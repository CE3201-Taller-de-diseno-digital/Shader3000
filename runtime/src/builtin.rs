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
use core::{convert::TryInto, iter, ops::Deref};

use crate::{
    chrono::{Duration, Ticks},
    matrix::State,
    sys,
};

type List = Vec<bool>;
type Mat = Vec<Rc<Vec<bool>>>;

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
pub extern "C" fn builtin_new_mat() -> *mut Mat {
    Rc::into_raw(Rc::<Mat>::default()) as *mut _
}

#[no_mangle]
pub extern "C" fn builtin_ref_list(list: *mut List) {
    let list = unsafe { Rc::from_raw(list) };
    let clone = Rc::clone(&list);

    Rc::into_raw(list);
    Rc::into_raw(clone);
}

#[no_mangle]
pub extern "C" fn builtin_ref_mat(mat: *mut Mat) {
    let mat = unsafe { Rc::from_raw(mat) };
    let clone = Rc::clone(&mat);

    Rc::into_raw(mat);
    Rc::into_raw(clone);
}

#[no_mangle]
pub extern "C" fn builtin_drop_list(list: *mut List) {
    //dropea valor al tomar ownership
    unsafe {
        Rc::from_raw(list);
    }
}

#[no_mangle]
pub extern "C" fn builtin_drop_mat(mat: *mut Mat) {
    unsafe {
        Rc::from_raw(mat);
    }
}

#[no_mangle]
pub extern "C" fn builtin_cmp_list(first: *mut List, second: *mut List) -> bool {
    let (first, second) = unsafe { (&*first, &*second) };
    first == second
}

#[no_mangle]
pub extern "C" fn builtin_cmp_mat(first: *mut Mat, second: *mut Mat) -> bool {
    let (first, second) = unsafe { (&*first, &*second) };
    first == second
}

#[no_mangle]
pub extern "C" fn builtin_index_list(list: *mut List, index: isize) -> bool {
    let list = unsafe { &*list };
    list[try_usize(index)]
}

#[no_mangle]
pub extern "C" fn builtin_index_entry_mat(mat: *mut Mat, row: isize, column: isize) -> bool {
    let mat = unsafe { &*mat };
    mat[try_usize(row)][try_usize(column)]
}

#[no_mangle]
pub extern "C" fn builtin_index_row_mat(mat: *mut Mat, row: isize) -> *mut List {
    let mat = unsafe { &*mat };
    let row_list = Rc::clone(&mat[try_usize(row)]);
    Rc::into_raw(row_list) as *mut _
}

#[no_mangle]
pub extern "C" fn builtin_index_column_mat(mat: *mut Mat, column: isize) -> *mut List {
    let mat = unsafe { &*mat };
    let column = try_usize(column);

    let column_list = mat.iter().map(|row| row[column]).collect::<List>();
    Rc::into_raw(Rc::new(column_list)) as *mut _
}

#[no_mangle]
pub extern "C" fn builtin_insert_list(list: *mut List, index: isize, item: bool) {
    let list = unsafe { &mut *list };
    list.insert(try_usize(index), item);
}

#[no_mangle]
pub extern "C" fn builtin_insert_mat(mat: *mut Mat, item: *mut List, mode: isize, index: isize) {
    let mat = unsafe { &mut *mat };
    let original = unsafe { Rc::from_raw(item) };
    let item = Rc::clone(&original);
    Rc::into_raw(original);

    let index = try_usize(index);
    let row_count = mat.len();
    let column_count = mat.first().map(|first| first.len()).unwrap_or(0);

    match mode {
        0 => {
            assert!(
                row_count == 0 || column_count == item.len(),
                "attempted to insert row of length {} in {}x{} matrix",
                item.len(),
                row_count,
                column_count
            );

            mat.insert(index, item);
        }

        1 => {
            assert!(
                row_count == 0 || row_count == item.len(),
                "attempted to insert column of length {} in {}x{} matrix",
                item.len(),
                row_count,
                column_count
            );

            if row_count == 0 {
                (0..item.len()).for_each(|_| mat.push(Rc::new(List::new())));
            }

            for (row_list, entry) in mat.iter_mut().zip(item.iter().copied()) {
                let row_list = unsafe { Rc::get_mut_unchecked(row_list) };
                row_list.insert(index, entry);
            }
        }

        _ => panic!("bad matrix insertion mode: {}", mode),
    }
}

#[no_mangle]
pub extern "C" fn builtin_len_list(list: *mut List) -> isize {
    let list = unsafe { &*list };
    list.len() as isize
}

#[no_mangle]
pub extern "C" fn builtin_len_mat(mat: *mut Mat) -> isize {
    let mat = unsafe { &*mat };
    mat.len() as isize
}

#[no_mangle]
pub extern "C" fn builtin_slice_list(list: *mut List, from: isize, to: isize) -> *mut List {
    let list = unsafe { &*list };
    let slice = (&list[try_usize(from)..try_usize(to)]).to_vec();
    Rc::into_raw(Rc::new(slice)) as *mut _
}

#[no_mangle]
pub extern "C" fn builtin_slice_mat(mat: *mut List, from: isize, to: isize) -> *mut Mat {
    let mat = unsafe { &*mat };
    let slice = (&mat[try_usize(from)..try_usize(to)]).to_vec();
    Rc::into_raw(Rc::new(slice)) as *mut _
}

#[no_mangle]
pub extern "C" fn builtin_range(length: isize, value: bool) -> *mut List {
    let list = (0..length).map(|_| value).collect::<List>();
    Rc::into_raw(Rc::new(list)) as *mut _
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
    let list = unsafe { &*list };

    sys::with_display(|display| {
        for (col, value) in list_bits(&list) {
            display[(row, col)].set(State::from_bool(value));
        }
    });
}

#[no_mangle]
pub extern "C" fn builtin_printledx_c(col: isize, list: *mut List) {
    let list = unsafe { &*list };

    sys::with_display(|display| {
        for (row, value) in list_bits(&list) {
            display[(row, col)].set(State::from_bool(value));
        }
    });
}

#[no_mangle]
pub extern "C" fn builtin_printledx_m(index: isize, mat: *mut Mat) {
    assert!(
        index == 0,
        "PrintLedX(\"M\", index, ...) requires index 0, found {}",
        index
    );

    let mat = unsafe { &*mat };
    sys::with_display(|display| {
        for (row, col, value) in mat_bits(&mat) {
            display[(row, col)].set(State::from_bool(value));
        }
    });
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

fn mat_bits(mat: &[Rc<List>]) -> impl '_ + Iterator<Item = (isize, isize, bool)> {
    const EMPTY_ROW: &'static [bool] = &[false; 8];
    mat.iter()
        .map(Rc::deref)
        .map(Vec::as_slice)
        .chain(iter::repeat(EMPTY_ROW))
        .enumerate()
        .map(|(row, row_bits)| list_bits(row_bits).map(move |(col, bit)| (row as isize, col, bit)))
        .flatten()
        .take(8 * 8)
}

fn try_usize(as_isize: isize) -> usize {
    as_isize
        .try_into()
        .expect("attempted to use negative integer as index")
}
