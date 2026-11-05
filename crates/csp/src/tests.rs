#[test]
fn accepts_only_valid_tcp_port_numbers() {
    assert_eq!(super::parse_port("1"), Some(1));
    assert_eq!(super::parse_port("65535"), Some(65535));
    for invalid in ["0", "65536", "-42", "+42", "invalid", "", " 42"] {
        assert_eq!(super::parse_port(invalid), None);
    }
}

#[cfg(windows)]
mod windows {
    use crate::port::{confirmed_pid, listeners, verify_listener};
    use std::net::{Ipv4Addr, Ipv6Addr, TcpListener, TcpStream};
    use std::time::{Duration, Instant};

    fn wait_until(mut ready: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !ready() {
            assert!(
                Instant::now() < deadline,
                "la tabella TCP non si è aggiornata"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn finds_temporary_ipv4_listener() {
        let socket = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = socket.local_addr().unwrap();
        wait_until(|| {
            listeners(address.port())
                .unwrap()
                .iter()
                .any(|entry| entry.pid == std::process::id() && entry.address == address)
        });
    }

    #[test]
    fn finds_temporary_ipv6_listener() {
        let socket = TcpListener::bind((Ipv6Addr::LOCALHOST, 0)).unwrap();
        let address = socket.local_addr().unwrap();
        wait_until(|| {
            listeners(address.port())
                .unwrap()
                .iter()
                .any(|entry| entry.pid == std::process::id() && entry.address == address)
        });
    }

    #[test]
    fn rejects_an_owner_whose_listener_has_closed() {
        let socket = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = socket.local_addr().unwrap().port();
        let pid = std::process::id();
        wait_until(|| verify_listener(port, pid).is_ok());
        drop(socket);
        wait_until(|| verify_listener(port, pid).is_err());
    }

    #[test]
    fn excludes_an_established_client_connection() {
        let socket = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let client = TcpStream::connect(socket.local_addr().unwrap()).unwrap();
        let (_peer, _) = socket.accept().unwrap();
        wait_until(|| {
            verify_listener(socket.local_addr().unwrap().port(), std::process::id()).is_ok()
        });
        assert!(listeners(client.local_addr().unwrap().port())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn requires_explicit_confirmation_with_a_pid() {
        assert_eq!(confirmed_pid("termina 1234\r\n"), Some(1234));
        for invalid in [
            "",
            "s",
            "1234",
            "termina",
            "termina +1234",
            "termina 1234 5678",
        ] {
            assert_eq!(confirmed_pid(invalid), None);
        }
    }
}
