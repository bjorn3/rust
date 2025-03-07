use crate::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream};
use crate::os::net::linux_ext::tcp::TcpStreamExt;
use crate::sync::atomic::{AtomicUsize, Ordering};

static PORT: AtomicUsize = AtomicUsize::new(0);
const BASE_PORT: u16 = 19700; // Chosen to not conflict with tests/net/mod.rs

pub fn next_test_ip4() -> SocketAddr {
    let port = PORT.fetch_add(1, Ordering::Relaxed) as u16 + BASE_PORT;
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port))
}

#[test]
fn quickack() {
    macro_rules! t {
        ($e:expr) => {
            match $e {
                Ok(t) => t,
                Err(e) => panic!("received error for `{}`: {}", stringify!($e), e),
            }
        };
    }

    let addr = next_test_ip4();
    let _listener = t!(TcpListener::bind(&addr));

    let stream = t!(TcpStream::connect(&("localhost", addr.port())));

    t!(stream.set_quickack(false));
    assert_eq!(false, t!(stream.quickack()));
    t!(stream.set_quickack(true));
    assert_eq!(true, t!(stream.quickack()));
    t!(stream.set_quickack(false));
    assert_eq!(false, t!(stream.quickack()));
}

#[test]
#[cfg(target_os = "linux")]
fn deferaccept() {
    macro_rules! t {
        ($e:expr) => {
            match $e {
                Ok(t) => t,
                Err(e) => panic!("received error for `{}`: {}", stringify!($e), e),
            }
        };
    }

    let addr = next_test_ip4();
    let _listener = t!(TcpListener::bind(&addr));
    let stream = t!(TcpStream::connect(&("localhost", addr.port())));
    stream.set_deferaccept(1).expect("set_deferaccept failed");
    assert_eq!(stream.deferaccept().unwrap(), 1);
    stream.set_deferaccept(0).expect("set_deferaccept failed");
    assert_eq!(stream.deferaccept().unwrap(), 0);
}
