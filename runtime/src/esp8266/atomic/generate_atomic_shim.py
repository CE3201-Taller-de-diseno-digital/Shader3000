# How to use this script:
# 1) `$ python3 generate_atomic.shim.py`
#     - Generate with default params:
#     - Generated C file Path: atomic_shim.c
#     - Generated Rust file Path: atomic_shim.rs
#     - Proxy functions prefix: rust_xtensa
# 2) `$ python3 generate_atomic.shim.py <C file path> <Rust file path>`
# 3) `$ python3 generate_atomic.shim.py <C file path> <Rust file path> <Proxy functions prefix>`

import sys

DEFAULT_C_FILE_PATH = "atomic_shim.c"
DEFAULT_RUST_FILE_PATH = "atomic_shim.rs"
DEFAULT_FN_PREFIX = "rust_xtensa"

HEADER_COMMENT = "// THIS FILE IS GENERATED; PLEASE DO NOT CHANGE!\n"

OPERATION_FETCH_FUNCTIONS = [
    "__sync_fetch_and_add",
    "__sync_fetch_and_sub",
    "__sync_fetch_and_or",
    "__sync_fetch_and_and",
    "__sync_fetch_and_xor",
    "__sync_fetch_and_nand",
    "__sync_add_and_fetch",
    "__sync_sub_and_fetch",
    "__sync_or_and_fetch",
    "__sync_and_and_fetch",
    "__sync_xor_and_fetch",
    "__sync_nand_and_fetch",
    # Does not hold any operation, but signature is same
    "__sync_lock_test_and_set",
]

class SyncType:
    def __init__(self, size, c_type, rust_type):
        self.size = size
        self.c_type = c_type
        self.rust_type = rust_type

SYNC_TYPES = [
        SyncType(1, "int8_t", "i8"),
        SyncType(2, "int16_t", "i16"),
        SyncType(4, "int32_t", "i32"),
        SyncType(8, "int64_t", "i64")
]

class AtomicShimGenerator:
    def __init__(self, c_file_path, rust_file_path, fn_prefix):
        self.c_data = ""
        self.rust_data = ""

        self.c_file_path = c_file_path
        self.rust_file_path = rust_file_path
        self.fn_prefix = fn_prefix

    def __generate_c_header(self):
        self.c_data += HEADER_COMMENT + "#include <stdint.h>\n\n"

    def __generate_rust_header(self):
        self.rust_data += HEADER_COMMENT + "#![allow(non_snake_case)]\n\n"

    def __save(self):
        with open(self.c_file_path, 'w') as f:
            f.write(self.c_data)
        with open(self.rust_file_path, 'w') as f:
            f.write(self.rust_data)


    def generate(self):
        self.__generate_c_header()
        self.__generate_rust_header()

        for sync_type in SYNC_TYPES:
            for func in OPERATION_FETCH_FUNCTIONS:
                self.__c_generate_fetch_operation(func, sync_type.c_type, sync_type.size)
                self.__rust_generate_fetch_operation(func, sync_type.rust_type, sync_type.size)
            self.__c_generate_val_compare_and_swap(sync_type.c_type, sync_type.size)
            self.__rust_generate_val_compare_and_swap(sync_type.rust_type, sync_type.size)
            self.__c_generate_bool_compare_and_swap(sync_type.c_type, sync_type.size)
            self.__rust_generate_bool_compare_and_swap(sync_type.rust_type, sync_type.size)
        self.__save()


    def __rust_add_extern(self, value):
        self.rust_data += "extern \"C\" { " + value + " }\n\n"

    def __rust_add_builtin_surrogate(self, value):
        self.rust_data += "#[no_mangle]\n" + value + "\n\n"

    def __rust_generate_val_compare_and_swap(self, sync_type, suffix):
        FN_NAME = "__sync_val_compare_and_swap"

        extern_entry = "fn {3}{0}_{2}(ptr: *mut {1}, old: {1}, new: {1}) -> {1};"\
                .format(FN_NAME, sync_type, suffix, self.fn_prefix)
        self.__rust_add_extern(extern_entry)

        builtin_surrogate = ("unsafe fn {0}_{2}(ptr: *mut {1}, old: {1}, new: {1}) -> {1} {{\n"
            + "    {3}{0}_{2}(ptr, old, new)\n"
            + "}}").format(FN_NAME, sync_type, suffix, self.fn_prefix)
        self.__rust_add_builtin_surrogate(builtin_surrogate)

    def __rust_generate_bool_compare_and_swap(self, sync_type, suffix):
        FN_NAME = "__sync_bool_compare_and_swap"

        extern_entry = "fn {3}{0}_{2}(ptr: *mut {1}, old: {1}, new: {1}) -> bool;"\
                .format(FN_NAME, sync_type, suffix, self.fn_prefix)
        self.__rust_add_extern(extern_entry)

        builtin_surrogate = ("unsafe fn {0}_{2}(ptr: *mut {1}, old: {1}, new: {1}) -> bool {{\n"
            + "    {3}{0}_{2}(ptr, old, new)\n"
            + "}}").format(FN_NAME, sync_type, suffix, self.fn_prefix)
        self.__rust_add_builtin_surrogate(builtin_surrogate)

    def __rust_generate_fetch_operation(self, func, sync_type, suffix):
        extern_entry = "fn {3}{0}_{2}(ptr: *mut {1}, arg: {1}) -> {1};"\
                .format(func, sync_type, suffix, self.fn_prefix)
        self.__rust_add_extern(extern_entry)

        builtin_surrogate = ("unsafe fn {0}_{2}(ptr: *mut {1}, arg: {1}) -> {1} {{\n"
            + "    {3}{0}_{2}(ptr, arg)\n"
            + "}}").format(func, sync_type, suffix, self.fn_prefix)
        self.__rust_add_builtin_surrogate(builtin_surrogate)

    def __c_add_builtin_proxy(self, value):
        self.c_data += value + "\n\n"

    def __c_generate_fetch_operation(self, func, sync_type, suffix):
        builtin_proxy = ("{1} {3}{0}_{2}({1}* ptr, {1} arg) {{\n"
                + "    return {0}_{2}(ptr, arg);\n"
                + "}}").format(func, sync_type, suffix, self.fn_prefix)
        self.__c_add_builtin_proxy(builtin_proxy)


    def __c_generate_val_compare_and_swap(self, sync_type, suffix):
        FN_NAME = "__sync_val_compare_and_swap"
        builtin_proxy = ("{1} {3}{0}_{2}({1}* ptr, {1} old, {1} new) {{\n"
                + "    return {0}_{2}(ptr, old, new);\n"
                + "}}").format(FN_NAME, sync_type, suffix, self.fn_prefix)
        self.__c_add_builtin_proxy(builtin_proxy)

    def __c_generate_bool_compare_and_swap(self, sync_type, suffix):
        FN_NAME = "__sync_bool_compare_and_swap"
        builtin_proxy = ("_Bool {3}{0}_{2}({1}* ptr, {1} old, {1} new) {{\n"
                + "    return {0}_{2}(ptr, old, new);\n"
                + "}}").format(FN_NAME, sync_type, suffix, self.fn_prefix)
        self.__c_add_builtin_proxy(builtin_proxy)

def main():
    c_file_path = DEFAULT_C_FILE_PATH
    rust_file_path = DEFAULT_RUST_FILE_PATH
    fn_prefix = DEFAULT_FN_PREFIX

    if len(sys.argv) >= 3:
        c_file_path = sys.argv[1]
        rust_file_path = sys.argv[2]

    if len(sys.argv) >= 4:
        fn_prefix = sys.argv[3]

    AtomicShimGenerator(c_file_path, rust_file_path, fn_prefix).generate()

main()
