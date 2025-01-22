pub fn smol_now() -> smoltcp::time::Instant {
    smoltcp::time::Instant::from_millis(u64::from(lilos::time::TickTime::now()) as i64)
}
