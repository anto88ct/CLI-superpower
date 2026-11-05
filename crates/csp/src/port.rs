use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{self, IsTerminal, Write};
use std::mem::{offset_of, size_of, size_of_val};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6};
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::{
    ERROR_INSUFFICIENT_BUFFER, FILETIME, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID, MIB_TCPROW_OWNER_PID,
    MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
};
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, IsProcessCritical, OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
    WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Listener {
    pub(crate) pid: u32,
    pub(crate) address: SocketAddr,
}

pub(crate) fn listeners(port: u16) -> io::Result<Vec<Listener>> {
    let ipv4 = tcp_table(AF_INET)?;
    let ipv6 = tcp_table(AF_INET6)?;
    let mut listeners = Vec::new();
    // SAFETY: the AF_INET table contains only C MIB_TCPROW_OWNER_PID rows.
    let rows4 =
        unsafe { rows::<MIB_TCPROW_OWNER_PID>(&ipv4, offset_of!(MIB_TCPTABLE_OWNER_PID, table)) }?;
    for row in rows4 {
        // The table class selects listeners. Some Windows builds report
        // inconsistent dwState values even for an open listening socket.
        if row.dwRemoteAddr == 0 && row.dwRemotePort == 0 && network_port(row.dwLocalPort) == port {
            listeners.push(Listener {
                pid: row.dwOwningPid,
                address: (Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes()), port).into(),
            });
        }
    }
    // SAFETY: the AF_INET6 table contains only C MIB_TCP6ROW_OWNER_PID rows.
    let rows6 = unsafe {
        rows::<MIB_TCP6ROW_OWNER_PID>(&ipv6, offset_of!(MIB_TCP6TABLE_OWNER_PID, table))
    }?;
    for row in rows6 {
        if row.ucRemoteAddr == [0; 16]
            && row.dwRemotePort == 0
            && network_port(row.dwLocalPort) == port
        {
            listeners.push(Listener {
                pid: row.dwOwningPid,
                address: SocketAddrV6::new(
                    Ipv6Addr::from(row.ucLocalAddr),
                    port,
                    0,
                    row.dwLocalScopeId,
                )
                .into(),
            });
        }
    }
    listeners.sort_unstable();
    listeners.dedup();
    Ok(listeners)
}

fn network_port(value: u32) -> u16 {
    u16::from_be(value as u16)
}

fn tcp_table(family: u16) -> io::Result<Vec<u32>> {
    // The documented table consists of DWORDs and byte arrays. Use DWORD-aligned storage.
    let mut buffer = Vec::<u32>::new();
    let mut size = 0;
    for _ in 0..5 {
        let pointer = if buffer.is_empty() {
            std::ptr::null_mut()
        } else {
            buffer.as_mut_ptr().cast()
        };
        // SAFETY: pointer is null for sizing or points to an aligned buffer of at least size bytes.
        let result = unsafe {
            GetExtendedTcpTable(
                pointer,
                &mut size,
                0,
                u32::from(family),
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            )
        };
        match result {
            0 => return Ok(buffer),
            ERROR_INSUFFICIENT_BUFFER => {
                buffer.resize((size as usize).div_ceil(size_of::<u32>()), 0)
            }
            code => return Err(io::Error::from_raw_os_error(code as i32)),
        }
    }
    Err(io::Error::other("la tabella TCP cambia troppo rapidamente"))
}

/// The caller must select the actual table row type, which must allow every bit pattern.
unsafe fn rows<T: Copy>(table: &[u32], offset: usize) -> io::Result<impl Iterator<Item = T> + '_> {
    let count = table.first().copied().unwrap_or(0) as usize;
    let bytes = size_of_val(table);
    if count > bytes.saturating_sub(offset) / size_of::<T>() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "tabella TCP troncata",
        ));
    }
    let base = table.as_ptr().cast::<u8>();
    Ok((0..count).map(move |index| {
        // SAFETY: the caller selects the corresponding C row type and offset for the API table.
        // The bounds check above covers each row, whose fields all admit every bit pattern.
        unsafe {
            base.add(offset + index * size_of::<T>())
                .cast::<T>()
                .read_unaligned()
        }
    }))
}

pub(crate) fn run(port: u16) -> Result<(), String> {
    let listeners = listeners(port).map_err(|e| format!("lettura listener TCP: {e}"))?;
    if listeners.is_empty() {
        println!("Nessun listener TCP sulla porta {port}.");
        return Ok(());
    }

    let mut processes = BTreeMap::new();
    for listener in listeners {
        processes
            .entry(listener.pid)
            .or_insert_with(|| (Process::open(listener.pid, false), Vec::new()))
            .1
            .push(listener.address);
    }
    let mut terminable = Vec::new();
    for (&pid, (process, endpoints)) in &processes {
        let endpoints = endpoints
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        match process {
            Ok(process) => {
                println!("PID {pid} | {endpoints}");
                match process.image_path() {
                    Ok(path) => {
                        let name = path.file_name().unwrap_or(path.as_os_str());
                        println!("  Nome: {}", name.to_string_lossy());
                        println!("  Percorso: {}", path.display());
                    }
                    Err(error) => println!("  Nome e percorso non disponibili ({error})"),
                }
                match process.created_at() {
                    Ok(created) => {
                        if let Some(age) = process_age(created) {
                            println!("  In esecuzione da {age}");
                        }
                        if pid != std::process::id() {
                            terminable.push((pid, created));
                        }
                    }
                    Err(error) => println!("  Ora di avvio non disponibile ({error})"),
                }
            }
            Err(error) => println!("PID {pid} | {endpoints} | dettagli non disponibili ({error})"),
        }
    }
    if terminable.is_empty() || !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(());
    }

    print!("Per terminare un processo, digita termina <PID>. Invio per annullare: ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;
    if input.trim().is_empty() {
        return Ok(());
    }
    let pid = confirmed_pid(&input).ok_or("conferma non valida. Usa termina <PID>.")?;
    let (_, created) = terminable
        .into_iter()
        .find(|&(candidate, _)| candidate == pid)
        .ok_or("il PID non è tra i listener con identità verificata")?;
    terminate(port, pid, created)
}

pub(crate) fn confirmed_pid(input: &str) -> Option<u32> {
    let mut words = input.split_whitespace();
    match (words.next(), words.next(), words.next()) {
        (Some("termina"), Some(pid), None) if pid.bytes().all(|byte| byte.is_ascii_digit()) => {
            pid.parse().ok()
        }
        _ => None,
    }
}

struct Process(OwnedHandle);

impl Process {
    fn open(pid: u32, terminate: bool) -> io::Result<Self> {
        let rights = if terminate {
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | PROCESS_SYNCHRONIZE
        } else {
            PROCESS_QUERY_LIMITED_INFORMATION
        };
        // SAFETY: OpenProcess returns a uniquely owned handle on success, transferred immediately.
        let handle = unsafe { OpenProcess(rights, 0, pid) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(unsafe { OwnedHandle::from_raw_handle(handle) }))
    }

    fn image_path(&self) -> io::Result<PathBuf> {
        let mut path = vec![0u16; 32_768];
        let mut size = path.len() as u32;
        // SAFETY: handle is valid, and path has capacity for size UTF-16 code units.
        if unsafe {
            QueryFullProcessImageNameW(self.0.as_raw_handle(), 0, path.as_mut_ptr(), &mut size)
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(OsString::from_wide(&path[..size as usize]).into())
    }

    fn created_at(&self) -> io::Result<u64> {
        let [mut created, mut exit, mut kernel, mut user] = [FILETIME::default(); 4];
        // SAFETY: handle is valid and the four output pointers are distinct initialized FILETIMEs.
        if unsafe {
            GetProcessTimes(
                self.0.as_raw_handle(),
                &mut created,
                &mut exit,
                &mut kernel,
                &mut user,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
    }

    fn is_critical(&self) -> io::Result<bool> {
        let mut critical = 0;
        // SAFETY: handle is valid, and critical is writable.
        if unsafe { IsProcessCritical(self.0.as_raw_handle(), &mut critical) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(critical != 0)
    }
}

fn process_age(created: u64) -> Option<String> {
    const UNIX_EPOCH_FILETIME: u64 = 116_444_736_000_000_000;
    let seconds_since_epoch = created.checked_sub(UNIX_EPOCH_FILETIME)? / 10_000_000;
    let start = UNIX_EPOCH.checked_add(Duration::from_secs(seconds_since_epoch))?;
    let seconds = SystemTime::now().duration_since(start).ok()?.as_secs();
    Some(format!(
        "{}g {:02}h {:02}m {:02}s",
        seconds / 86_400,
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    ))
}

pub(crate) fn verify_listener(port: u16, pid: u32) -> Result<(), String> {
    let current = listeners(port).map_err(|e| format!("nuova lettura listener TCP: {e}"))?;
    if current.iter().any(|listener| listener.pid == pid) {
        Ok(())
    } else {
        Err(format!(
            "il PID {pid} non è più in ascolto sulla porta {port}"
        ))
    }
}

fn terminate(port: u16, pid: u32, confirmed_creation: u64) -> Result<(), String> {
    let process = Process::open(pid, true)
        .map_err(|e| format!("apertura del PID {pid} per la terminazione: {e}"))?;
    let current_creation = process
        .created_at()
        .map_err(|e| format!("verifica dell'identità del PID {pid}: {e}"))?;
    if current_creation != confirmed_creation {
        return Err(format!("il PID {pid} identifica ora un processo diverso"));
    }
    if process
        .is_critical()
        .map_err(|e| format!("verifica del PID {pid}: {e}"))?
    {
        return Err(format!("il PID {pid} è un processo critico di Windows"));
    }
    verify_listener(port, pid)?;
    let handle = process.0.as_raw_handle();
    // SAFETY: the owned handle has termination and synchronization rights and targets the verified process.
    if unsafe { TerminateProcess(handle, 1) } == 0 {
        return Err(format!(
            "terminazione del PID {pid}: {}",
            io::Error::last_os_error()
        ));
    }
    // SAFETY: handle remains valid and owned while waiting.
    match unsafe { WaitForSingleObject(handle, 5_000) } {
        WAIT_OBJECT_0 => {
            println!("Processo {pid} terminato.");
            Ok(())
        }
        WAIT_TIMEOUT => Err(format!(
            "terminazione richiesta per il PID {pid}, ancora in attesa dopo 5 secondi"
        )),
        WAIT_FAILED => Err(format!(
            "attesa della terminazione del PID {pid}: {}",
            io::Error::last_os_error()
        )),
        result => Err(format!(
            "attesa della terminazione del PID {pid}: esito inatteso {result}"
        )),
    }
}
