#![cfg(not(any(target_os = "emscripten", all(target_os = "wasi", target_env = "p1"))))]

use std::net::Ipv4Addr;

use crate::{sa4, tsa};

#[test]
fn to_socket_addr_socketaddr() {
    let a = sa4(Ipv4Addr::new(77, 88, 21, 11), 12345);
    assert_eq!(Ok(vec![a]), tsa(a));
}
