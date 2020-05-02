extern crate wast_spec;
use std::path::Path;
use wast_spec::WastContext;

macro_rules! run_wast {
    ($file:expr, $func_name:ident) => {
        #[test]
        fn $func_name() {
            run_spectest($file)
        }
    };
}

fn run_spectest(filename: &str) {
    let testsuite_dir = Path::new(file!()).parent().unwrap().join("testsuite");
    let mut context = WastContext::new();
    match context.run_file(&testsuite_dir.join(filename)) {
        Ok(_) => (),
        Err(err) => panic!("{}", err),
    }
}

run_wast!("address.wast", test_wast_address);
run_wast!("align.wast", test_wast_align);
run_wast!("binary-leb128.wast", test_wast_binary_leb128);
run_wast!("binary.wast", test_wast_binary);
run_wast!("block.wast", test_wast_block);
run_wast!("br.wast", test_wast_br);
run_wast!("br_if.wast", test_wast_br_if);
run_wast!("br_table.wast", test_wast_br_table);
run_wast!("break-drop.wast", test_wast_break_drop);
run_wast!("call.wast", test_wast_call);
run_wast!("call_indirect.wast", test_wast_call_indirect);
run_wast!("comments.wast", test_wast_comments);
run_wast!("const.wast", test_wast_const);
run_wast!("conversions.wast", test_wast_conversions);
run_wast!("custom.wast", test_wast_custom);
run_wast!("data.wast", test_wast_data);
run_wast!("elem.wast", test_wast_elem);
run_wast!("endianness.wast", test_wast_endianness);
run_wast!("exports.wast", test_wast_exports);
run_wast!("f32.wast", test_wast_f32);
run_wast!("f32_bitwise.wast", test_wast_f32_bitwise);
run_wast!("f32_cmp.wast", test_wast_f32_cmp);
run_wast!("f64.wast", test_wast_f64);
run_wast!("f64_bitwise.wast", test_wast_f64_bitwise);
run_wast!("f64_cmp.wast", test_wast_f64_cmp);
run_wast!("fac.wast", test_wast_fac);
run_wast!("float_exprs.wast", test_wast_float_exprs);
run_wast!("float_literals.wast", test_wast_float_literals);
run_wast!("float_memory.wast", test_wast_float_memory);
run_wast!("float_misc.wast", test_wast_float_misc);
run_wast!("forward.wast", test_wast_forward);
run_wast!("func.wast", test_wast_func);
run_wast!("func_ptrs.wast", test_wast_func_ptrs);
run_wast!("globals.wast", test_wast_globals);
run_wast!("i32.wast", test_wast_i32);
run_wast!("i64.wast", test_wast_i64);
run_wast!("if.wast", test_wast_if);
run_wast!("imports.wast", test_wast_imports);
run_wast!("inline-module.wast", test_wast_inline_module);
run_wast!("int_exprs.wast", test_wast_int_exprs);
run_wast!("int_literals.wast", test_wast_int_literals);
run_wast!("labels.wast", test_wast_labels);
run_wast!("left-to-right.wast", test_wast_left_to_right);
run_wast!("linking.wast", test_wast_linking);
run_wast!("load.wast", test_wast_load);
run_wast!("local_get.wast", test_wast_local_get);
run_wast!("local_set.wast", test_wast_local_set);
run_wast!("local_tee.wast", test_wast_local_tee);
run_wast!("loop.wast", test_wast_loop);
run_wast!("memory.wast", test_wast_memory);
run_wast!("memory_grow.wast", test_wast_memory_grow);
run_wast!("memory_redundancy.wast", test_wast_memory_redundancy);
run_wast!("memory_size.wast", test_wast_memory_size);
run_wast!("memory_trap.wast", test_wast_memory_trap);
run_wast!("names.wast", test_wast_names);
run_wast!("nop.wast", test_wast_nop);
run_wast!("return.wast", test_wast_return);
run_wast!("select.wast", test_wast_select);
run_wast!("skip-stack-guard-page.wast", test_wast_skip_stack_guard_page);
run_wast!("stack.wast", test_wast_stack);
run_wast!("start.wast", test_wast_start);
run_wast!("store.wast", test_wast_store);
run_wast!("switch.wast", test_wast_switch);
run_wast!("token.wast", test_wast_token);
run_wast!("traps.wast", test_wast_traps);
run_wast!("type.wast", test_wast_type);
run_wast!("typecheck.wast", test_wast_typecheck);
run_wast!("unreachable.wast", test_wast_unreachable);
run_wast!("unreached-invalid.wast", test_wast_unreached_invalid);
run_wast!("unwind.wast", test_wast_unwind);
run_wast!("utf8-custom-section-id.wast", test_wast_utf8_custom_section_id);
run_wast!("utf8-import-field.wast", test_wast_utf8_import_field);
run_wast!("utf8-import-module.wast", test_wast_utf8_import_module);
run_wast!("utf8-invalid-encoding.wast", test_wast_utf8_invalid_encoding);
