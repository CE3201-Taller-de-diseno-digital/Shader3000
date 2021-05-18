pub fn debug(hint: usize) {
    dbg!(hint);
}

pub fn delay_ms(millis: u32) {
    std::thread::sleep(std::time::Duration::from_millis(millis as u64));
}
