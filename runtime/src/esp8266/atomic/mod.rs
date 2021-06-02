// THIS FILE IS GENERATED; PLEASE DO NOT CHANGE!
#![allow(non_snake_case)]

use xtensa_lx::interrupt::free;

#[no_mangle]
unsafe extern "C" fn __sync_val_compare_and_swap_1(ptr: *mut i8, old: i8, new: i8) -> i8 {
    free(|_| {
        let last = *ptr;
        if last == old {
            *ptr = new;
        }

        last
    })
}
