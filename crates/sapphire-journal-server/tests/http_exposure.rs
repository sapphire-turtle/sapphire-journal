//! `check_exposure`: a non-loopback bind with no `allowed_hosts` would answer
//! every `/mcp` request with 403, so the server refuses to start that way.

use sapphire_journal_server::check_exposure;

#[test]
fn loopback_needs_no_allowed_hosts() {
    assert!(check_exposure("127.0.0.1:3172".parse().unwrap(), &[]).is_ok());
    assert!(check_exposure("[::1]:3172".parse().unwrap(), &[]).is_ok());
}

#[test]
fn a_wider_bind_needs_a_host_name() {
    let addr = "0.0.0.0:3172".parse().unwrap();
    assert!(check_exposure(addr, &[]).is_err());
    assert!(check_exposure(addr, &["  ".to_owned()]).is_err());
    assert!(check_exposure(addr, &["box.tailnet.ts.net".to_owned()]).is_ok());
}
