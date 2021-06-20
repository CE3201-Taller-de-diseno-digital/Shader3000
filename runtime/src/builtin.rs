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

#[cfg(target_arch = "xtensa")]
use micromath::F32Ext;

use crate::{
    chrono::{Duration, Ticks},
    matrix::State,
    sys,
};

type List = Vec<bool>;
type Mat = Vec<Rc<List>>;

enum Orientation {
    Rows,
    Columns,
}

#[no_mangle]
pub extern "C" fn builtin_debug(line: isize) {
    sys_debug!("[line {}] builtin_debug()", line);
}

#[no_mangle]
pub extern "C" fn builtin_debug_bool(line: isize, hint: bool) {
    sys_debug!("[line {}] builtin_debug_bool({:?})", line, hint);
}

#[no_mangle]
pub extern "C" fn builtin_debug_int(line: isize, hint: isize) {
    sys_debug!("[line {}] builtin_debug_int({})", line, hint);
}

#[no_mangle]
pub extern "C" fn builtin_debug_float(line: isize, hint: isize) {
    sys_debug!(
        "[line {}] builtin_debug_float({})",
        line,
        f32_from_ffi(hint)
    );
}

#[no_mangle]
pub extern "C" fn builtin_debug_list(line: isize, list: *mut List) {
    let list = unsafe { &*list };
    sys_debug!("[line {}] builtin_debug_list({:?})", line, list);
}

#[no_mangle]
pub extern "C" fn builtin_debug_mat(line: isize, mat: *mut Mat) {
    let mat = unsafe { &*mat };
    sys_debug!("[line {}] builtin_debug_mat({:?})", line, mat);
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
pub extern "C" fn builtin_eq_list(first: *mut List, second: *mut List) -> isize {
    let (first, second) = unsafe { (&*first, &*second) };
    bool_to_ffi(first == second)
}

#[no_mangle]
pub extern "C" fn builtin_eq_mat(first: *mut Mat, second: *mut Mat) -> isize {
    let (first, second) = unsafe { (&*first, &*second) };
    bool_to_ffi(first == second)
}

#[no_mangle]
pub extern "C" fn builtin_index_list(list: *mut List, index: isize) -> isize {
    let list = unsafe { &*list };
    bool_to_ffi(list[try_usize(index)])
}

#[no_mangle]
pub extern "C" fn builtin_index_entry_mat(mat: *mut Mat, row: isize, column: isize) -> isize {
    let mat = unsafe { &*mat };
    bool_to_ffi(mat[try_usize(row)][try_usize(column)])
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
pub extern "C" fn builtin_insert_mat(mat: *mut Mat, vectors: *mut Mat, mode: isize, index: isize) {
    let (mat, vectors) = unsafe { (&mut *mat, &*vectors) };
    insert_in_mat(mat, vectors, mode, try_usize(index));
}

#[no_mangle]
pub extern "C" fn builtin_insert_end_mat(mat: *mut Mat, vectors: *mut Mat, mode: isize) {
    let (mat, vectors) = unsafe { (&mut *mat, &*vectors) };
    let length = match try_orientation(mode) {
        Orientation::Rows => shapef(mat),
        Orientation::Columns => shapec(mat),
    };

    insert_in_mat(mat, vectors, mode, length);
}

#[no_mangle]
pub extern "C" fn builtin_delete_list(list: *mut List, index: isize) {
    let list = unsafe { &mut *list };
    list.remove(try_usize(index));
}

#[no_mangle]
pub extern "C" fn builtin_delete_mat(mat: *mut Mat, index: isize, mode: isize) {
    let mat = unsafe { &mut *mat };

    let index = try_usize(index);
    match try_orientation(mode) {
        Orientation::Rows => drop(mat.remove(index)),
        Orientation::Columns => mat.iter_mut().for_each(|row| {
            let row = unsafe { Rc::get_mut_unchecked(row) };
            row.remove(index);
        }),
    }
}

#[no_mangle]
pub extern "C" fn builtin_push_mat(mat: *mut Mat, item: *mut List) {
    let (mat, item) = unsafe { (&mut *mat, Rc::from_raw(item)) };

    insert_in_mat(mat, core::slice::from_ref(&item), 0, mat.len());
    Rc::into_raw(item);
}

#[no_mangle]
pub extern "C" fn builtin_len_list(list: *mut List) -> isize {
    let list = unsafe { &*list };
    list.len() as isize
}

// No hay builtin_len_mat(), en vez de eso se tiene builtin_shapef()

#[no_mangle]
pub extern "C" fn builtin_slice_list(list: *mut List, from: isize, to: isize) -> *mut List {
    let list = unsafe { &*list };
    let slice = (&list[try_usize(from)..try_usize(to)]).to_vec();
    Rc::into_raw(Rc::new(slice)) as *mut _
}

#[no_mangle]
pub extern "C" fn builtin_slice_mat(mat: *mut Mat, from: isize, to: isize) -> *mut Mat {
    let mat = unsafe { &*mat };
    let slice = (&mat[try_usize(from)..try_usize(to)]).to_vec();
    Rc::into_raw(Rc::new(slice)) as *mut _
}

#[no_mangle]
pub extern "C" fn builtin_set_entry_list(list: *mut List, index: isize, entry: bool) {
    let list = unsafe { &mut *list };
    list[try_usize(index)] = entry;
}

#[no_mangle]
pub extern "C" fn builtin_set_entry_mat(mat: *mut Mat, row: isize, col: isize, entry: bool) {
    let mat = unsafe { &mut *mat };
    let row = unsafe { Rc::get_mut_unchecked(&mut mat[try_usize(row)]) };
    row[try_usize(col)] = entry;
}

#[no_mangle]
pub extern "C" fn builtin_set_row_mat(mat: *mut Mat, row: isize, entry: *mut List) {
    let (mat, entry) = unsafe { (&mut *mat, Rc::from_raw(entry)) };

    let shapec = shapec(mat);
    assert!(
        shapec == entry.len(),
        "attempted to replace row of length {} with list of length {}",
        shapec,
        entry.len()
    );

    mat[try_usize(row)] = Rc::clone(&entry);
    Rc::into_raw(entry);
}

#[no_mangle]
pub extern "C" fn builtin_set_column_mat(mat: *mut Mat, column: isize, entry: *mut List) {
    let (mat, entry) = unsafe { (&mut *mat, &*entry) };

    let shapef = shapef(mat);
    assert!(
        shapef == entry.len(),
        "attempted to replace column of length {} with list of length {}",
        shapef,
        entry.len()
    );

    let column = try_usize(column);
    for (row, value) in mat.iter_mut().zip(entry.iter().cloned()) {
        let row = unsafe { Rc::get_mut_unchecked(row) };
        row[column] = value;
    }
}

#[no_mangle]
pub extern "C" fn builtin_set_slice_list(
    list: *mut List,
    from: isize,
    to: isize,
    values: *mut List,
) {
    let (list, values) = unsafe { (&mut *list, &*values) };

    let target = &mut list[try_usize(from)..try_usize(to)];
    assert!(target.len() == values.len());

    target
        .iter_mut()
        .zip(values.iter().copied())
        .for_each(|(entry, value)| *entry = value);
}

#[no_mangle]
pub extern "C" fn builtin_set_slice_mat(mat: *mut Mat, from: isize, to: isize, rows: *mut Mat) {
    let (mat, rows) = unsafe { (&mut *mat, &*rows) };

    let (target_shapec, source_shapec) = (shapec(mat), shapec(rows));
    assert!(
        source_shapec == target_shapec,
        "attempted to replace matrix slice of {} columns with matrix of {} columns",
        target_shapec,
        source_shapec
    );

    let target = &mut mat[try_usize(from)..try_usize(to)];
    assert!(target.len() == rows.len());

    for (target_row, source_row) in target.iter_mut().zip(rows.iter()) {
        *target_row = source_row.clone();
    }
}

#[no_mangle]
pub extern "C" fn builtin_shapef(mat: *mut Mat) -> isize {
    let mat = unsafe { &*mat };
    shapef(mat) as isize
}

#[no_mangle]
pub extern "C" fn builtin_shapec(mat: *mut Mat) -> isize {
    let mat = unsafe { &*mat };
    shapec(mat) as isize
}

#[no_mangle]
pub extern "C" fn builtin_range(length: isize, value: bool) -> *mut List {
    let list = (0..length).map(|_| value).collect::<List>();
    Rc::into_raw(Rc::new(list)) as *mut _
}

#[no_mangle]
pub extern "C" fn builtin_cast_int_float(integer: isize) -> isize {
    f32_to_ffi(integer as f32)
}

#[no_mangle]
pub extern "C" fn builtin_cast_float_int(float: isize) -> isize {
    f32_from_ffi(float) as isize
}

#[no_mangle]
pub extern "C" fn builtin_div_int(a: isize, b: isize) -> isize {
    f32_to_ffi((a as f32) / (b as f32))
}

#[no_mangle]
pub extern "C" fn builtin_pow_int(a: isize, b: isize) -> isize {
    f32_to_ffi((a as f32).powf(b as f32))
}

#[no_mangle]
pub extern "C" fn builtin_add_float(a: isize, b: isize) -> isize {
    f32_to_ffi(f32_from_ffi(a) + f32_from_ffi(b))
}

#[no_mangle]
pub extern "C" fn builtin_sub_float(a: isize, b: isize) -> isize {
    f32_to_ffi(f32_from_ffi(a) - f32_from_ffi(b))
}

#[no_mangle]
pub extern "C" fn builtin_mul_float(a: isize, b: isize) -> isize {
    f32_to_ffi(f32_from_ffi(a) * f32_from_ffi(b))
}

#[no_mangle]
pub extern "C" fn builtin_div_float(a: isize, b: isize) -> isize {
    f32_to_ffi(f32_from_ffi(a) / f32_from_ffi(b))
}

#[no_mangle]
pub extern "C" fn builtin_pow_float(a: isize, b: isize) -> isize {
    f32_to_ffi(f32_from_ffi(a).powf(f32_from_ffi(b)))
}

#[no_mangle]
pub extern "C" fn builtin_cmp_float(a: isize, b: isize) -> isize {
    use core::cmp::Ordering::*;

    //FIXME
    match f32_from_ffi(a)
        .partial_cmp(&f32_from_ffi(b))
        .unwrap_or(Greater)
    {
        Less => -1,
        Equal => 0,
        Greater => 1,
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

fn f32_from_ffi(arg: isize) -> f32 {
    f32::from_bits(arg as u32)
}

fn f32_to_ffi(float: f32) -> isize {
    float.to_bits() as isize
}

fn bool_to_ffi(boolean: bool) -> isize {
    boolean as isize
}

fn shapef(mat: &Mat) -> usize {
    mat.len()
}

fn shapec(mat: &Mat) -> usize {
    mat.first().map(|row| row.len()).unwrap_or(0)
}

fn insert_in_mat(mat: &mut Mat, vectors: &[Rc<List>], mode: isize, index: usize) {
    // Formalmente, para este punto ya ocurrió UB si la condición de
    // aserción es falsa, ya que significa una violación de las reglas
    // de ownership y borrowing. Es posible que esto sea un probleam
    // cuando rustc utilice el atributo noalias. La solución correcta
    // es realizar esta verificación antes de dereferenciar ambos.
    assert!(
        mat.as_slice() as *const _ != vectors as *const _,
        "attempted to insert matrix into itself"
    );

    let row_count = shapef(mat);
    let column_count = shapec(mat);
    let mut corrected_rows = false;

    for item in vectors.iter().rev() {
        match try_orientation(mode) {
            Orientation::Rows => {
                assert!(
                    row_count == 0 || column_count == item.len(),
                    "attempted to insert row of length {} in {}x{} matrix",
                    item.len(),
                    row_count,
                    column_count
                );

                mat.insert(index, Rc::new(List::clone(item)));
            }

            Orientation::Columns => {
                assert!(
                    row_count == 0 || row_count == item.len(),
                    "attempted to insert column of length {} in {}x{} matrix",
                    item.len(),
                    row_count,
                    column_count
                );

                if row_count == 0 && !corrected_rows {
                    (0..item.len()).for_each(|_| mat.push(Rc::new(List::new())));
                    corrected_rows = true;
                }

                for (row_list, entry) in mat.iter_mut().zip(item.iter().copied()) {
                    let row_list = unsafe { Rc::get_mut_unchecked(row_list) };
                    row_list.insert(index, entry);
                }
            }
        }
    }
}

fn try_orientation(mode: isize) -> Orientation {
    match mode {
        0 => Orientation::Rows,
        1 => Orientation::Columns,
        _ => panic!("bad matrix insertion mode: {}", mode),
    }
}
