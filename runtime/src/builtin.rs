//! Interfaz pública.
//!
//! Las funciones en este módulo están diseñadas para ser
//! invocadas en la forma descrita en la documentación
//! top-level de este crate. Es necesario que todas sean
//! tanto `#[no_mangle]` como `extern "C"`.
//!
//! # Aquí no hay magia
//! Para garantizar agnosticismo en varias partes donde
//! no se necesita conocer detalles de la plataforma. En
//! vez de eso, este módulo debe realizar a llamadas a
//! `sys::*` cuando se necesita una operación
//! que depende de la plataforma.

use alloc::{rc::Rc, vec::Vec};
use core::{convert::TryInto, iter};

use crate::{
    chrono::{Duration, Ticks},
    matrix::State,
    sys,
};

type List = Vec<bool>;
type Mat = Vec<Vec<bool>>;

/// Imprime información de epuración en alguna manera no especificada.
#[no_mangle]
pub extern "C" fn builtin_debug(hint: isize) {
    sys_debug!("builtin_debug(0x{:x})", hint);
}

#[no_mangle]
pub extern "C" fn builtin_new_list() -> *mut List {
    Rc::into_raw(Rc::<List>::default()) as *mut _
}

#[no_mangle]
pub extern "C" fn builtin_drop_list(list: *mut List) {
    //dropea valor al tomar ownership
    unsafe {
        Rc::from_raw(list);
    }
}

/// Detiene el programa por una cantidad de milisegundos.
#[no_mangle]
pub extern "C" fn builtin_delay_mil(millis: isize) {
    sys::delay(milliseconds(millis));
}

/// Detiene el programa por una cantidad de segundos.
#[no_mangle]
pub extern "C" fn builtin_delay_seg(secs: isize) {
    sys::delay(seconds(secs));
}

/// Detiene el programa por una cantidad de minutos.
#[no_mangle]
pub extern "C" fn builtin_delay_min(mins: isize) {
    sys::delay(minutes(mins));
}

#[no_mangle]
pub extern "C" fn builtin_blink_mil(col: isize, row: isize, millis: isize, cond: bool) {
    blink(col, row, milliseconds(millis), cond);
}

#[no_mangle]
pub extern "C" fn builtin_blink_seg(col: isize, row: isize, secs: isize, cond: bool) {
    blink(col, row, seconds(secs), cond);
}

#[no_mangle]
pub extern "C" fn builtin_blink_min(col: isize, row: isize, mins: isize, cond: bool) {
    blink(col, row, minutes(mins), cond);
}

#[no_mangle]
pub extern "C" fn builtin_printled(col: isize, row: isize, value: bool) {
    sys::with_display(|display| {
        display[(row, col)].set(State::from_bool(value));
    });
}

#[no_mangle]
pub extern "C" fn builtin_printledx_f(row: isize, list: *mut List) {
    let list = unsafe { Rc::from_raw(list) };

    sys::with_display(|display| {
        for (col, value) in list_bits(&list) {
            display[(row, col)].set(State::from_bool(value));
        }
    });

    Rc::into_raw(list);
}

#[no_mangle]
pub extern "C" fn builtin_printledx_c(col: isize, list: *mut List) {
    let list = unsafe { Rc::from_raw(list) };

    sys::with_display(|display| {
        for (row, value) in list_bits(&list) {
            display[(row, col)].set(State::from_bool(value));
        }
    });

    Rc::into_raw(list);
}

#[no_mangle]
pub extern "C" fn builtin_printledx_m(index: isize, mat: *mut Mat) {
    assert!(
        index == 0,
        "PrintLedX(\"M\", index, ...) requires index 0, found {}",
        index
    );

    let mat = unsafe { Rc::from_raw(mat) };
    sys::with_display(|display| {
        for (row, col, value) in mat_bits(&mat) {
            display[(row, col)].set(State::from_bool(value));
        }
    });

    Rc::into_raw(mat);
}

fn blink(col: isize, row: isize, duration: Duration, cond: bool) {
    let allowed = 0..8;
    if allowed.contains(&col) && allowed.contains(&row) {
        let ticks = if cond {
            Ticks::from_duration(duration)
        } else {
            Ticks::default()
        };

        sys::with_display(|display| {
            display[(row, col)].blink(ticks);
        });
    }
}

fn milliseconds(millis: isize) -> Duration {
    Duration::from_millis(millis.try_into().unwrap_or_default())
}

fn seconds(secs: isize) -> Duration {
    Duration::from_secs(secs.try_into().unwrap_or_default())
}

fn minutes(mins: isize) -> Duration {
    let mins: u64 = mins.try_into().unwrap_or_default();
    Duration::from_secs(mins * 60)
}

fn list_bits(list: &[bool]) -> impl '_ + Iterator<Item = (isize, bool)> {
    list.iter()
        .copied()
        .chain(iter::repeat(false))
        .enumerate()
        .map(|(i, value)| (i as isize, value))
        .take(8)
}

fn mat_bits(mat: &[List]) -> impl '_ + Iterator<Item = (isize, isize, bool)> {
    const EMPTY_ROW: &'static [bool] = &[false; 8];
    mat.iter()
        .map(Vec::as_slice)
        .chain(iter::repeat(EMPTY_ROW))
        .enumerate()
        .map(|(row, row_bits)| list_bits(row_bits).map(move |(col, bit)| (row as isize, col, bit)))
        .flatten()
        .take(8)
}
