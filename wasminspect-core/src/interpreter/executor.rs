use super::address::{FuncAddr, GlobalAddr, MemoryAddr, TableAddr};
use super::func::*;
use super::host::BuiltinPrintI32;
use super::module::*;
use super::stack::*;
use super::store::*;
use super::value::*;
use parity_wasm::elements::{BlockType, FunctionType, InitExpr, Instruction, ValueType};

use std::convert::TryFrom;

#[derive(Debug)]
pub enum ExecError {
    Panic(String),
    NoCallFrame,
}

pub enum ExecSuccess {
    Next,
    End,
}

pub type ExecResult = Result<ExecSuccess, ExecError>;

#[derive(Debug)]
pub enum ReturnValError {
    TypeMismatchReturnValue(Value, ValueType),
    NoValue(ValueType),
    NoCallFrame,
}

pub type ReturnValResult = Result<Vec<Value>, ReturnValError>;

pub struct Executor<'a> {
    store: &'a mut Store,
    pc: ProgramCounter,
    stack: Stack,
}

impl<'a> Executor<'a> {
    pub fn new(
        local_len: usize,
        func_addr: FuncAddr,
        initial_args: Vec<Value>,
        initial_arity: usize,
        pc: ProgramCounter,
        store: &'a mut Store,
    ) -> Self {
        let mut stack = Stack::default();
        let frame = CallFrame::new(func_addr, local_len, initial_args, None);
        let f = CallFrame::new(func_addr, local_len, vec![], None);
        stack.set_frame(frame);
        stack.push_label(Label::Return(initial_arity));
        Self { store, pc, stack }
    }

    pub fn pop_result(&mut self, return_ty: Vec<ValueType>) -> ReturnValResult {
        let mut results = vec![];
        for ty in return_ty {
            let val = self.stack.pop_value();
            results.push(val);
            if val.value_type() != ty {
                return Err(ReturnValError::TypeMismatchReturnValue(val.clone(), ty));
            }
        }
        Ok(results)
    }

    pub fn current_func_insts(&self) -> &[Instruction] {
        let func = self.store.func(self.stack.current_func_addr());
        &func.defined().unwrap().code().instructions()
    }

    pub fn execute_step(&mut self) -> ExecResult {
        let func = self.store.func(self.pc.func_addr()).defined().unwrap();
        let module_index = func.module_index().clone();
        let inst = func.code().inst(self.pc.inst_index()).clone();
        return self.execute_inst(&inst, module_index);
    }

    fn execute_inst(&mut self, inst: &Instruction, module_index: ModuleIndex) -> ExecResult {
        self.pc.inc_inst_index();
        {
            let mut indent = String::new();
            for _ in 0..self.stack.current_frame_labels().len() {
                indent.push_str("  ");
            }
            println!("{}{}", indent, inst.clone());
        }
        println!("{:?}", self.stack);
        let result = match inst {
            Instruction::Unreachable => panic!(),
            Instruction::GetGlobal(index) => {
                let addr = GlobalAddr(module_index, *index as usize);
                let global = self.store.global(addr);
                self.stack.push_value(global.value());
                Ok(ExecSuccess::Next)
            }
            Instruction::SetGlobal(index) => {
                let addr = GlobalAddr(module_index, *index as usize);
                let value = self.stack.pop_value();
                self.store.set_global(addr, value);
                Ok(ExecSuccess::Next)
            }
            Instruction::SetLocal(index) => self.set_local(*index as usize),
            Instruction::GetLocal(index) => {
                let value = self.stack.current_frame().local(*index as usize);
                self.stack.push_value(value);
                Ok(ExecSuccess::Next)
            }
            Instruction::TeeLocal(index) => {
                let val = self.stack.pop_value();
                self.stack.push_value(val);
                self.stack.push_value(val);
                self.set_local(*index as usize)
            }
            Instruction::Drop => {
                self.stack.pop_value();
                Ok(ExecSuccess::Next)
            }
            Instruction::Select => {
                let cond: i32 = self.pop_as();
                let val2 = self.stack.pop_value();
                let val1 = self.stack.pop_value();
                if cond != 0 {
                    self.stack.push_value(val1);
                } else {
                    self.stack.push_value(val2);
                }
                Ok(ExecSuccess::Next)
            }
            Instruction::I32Const(val) => {
                self.stack.push_value(Value::I32(*val));
                Ok(ExecSuccess::Next)
            }
            Instruction::I32Add => self.int_int_op::<i32, _>(|a, b| Value::I32(a + b)),
            Instruction::I32Sub => self.int_int_op::<i32, _>(|a, b| Value::I32(a - b)),
            Instruction::I32Mul => self.int_int_op::<i32, _>(|a, b| Value::I32(a * b)),
            Instruction::I32Eq => {
                self.int_int_op::<i32, _>(|a, b| Value::I32(if a == b { 1 } else { 0 }))
            }
            Instruction::I32LtS => {
                self.int_int_op::<i32, _>(|a, b| Value::I32(if a < b { 1 } else { 0 }))
            }
            Instruction::I32Ctz => self.int_op::<i32, _>(|v| Value::I32(v.trailing_zeros() as i32)),
            Instruction::I32Eqz => {
                self.int_op::<i32, _>(|v| Value::I32(if v == 0 { 1 } else { 0 }))
            }
            Instruction::I64Const(val) => {
                self.stack.push_value(Value::I64(*val));
                Ok(ExecSuccess::Next)
            }
            Instruction::F32Const(val) => {
                self.stack.push_value(Value::F32(f32::from_bits(*val)));
                Ok(ExecSuccess::Next)
            }
            Instruction::F32Gt => {
                self.int_int_op::<f32, _>(|a, b| Value::I32(if a > b { 1 } else { 0 }))
            }
            Instruction::F64Const(val) => {
                self.stack.push_value(Value::F64(f64::from_bits(*val)));
                Ok(ExecSuccess::Next)
            }
            Instruction::I32Load(_, offset) => self.load::<i32>(*offset as usize),
            Instruction::I32Load8U(_, offset) => self.load_extend::<u8, i32>(*offset as usize),
            Instruction::I32Load16U(_, offset) => self.load_extend::<u16, i32>(*offset as usize),
            Instruction::I32Load8S(_, offset) => self.load_extend::<i8, i32>(*offset as usize),
            Instruction::I32Load16S(_, offset) => self.load_extend::<i16, i32>(*offset as usize),
            Instruction::I32Store(_, offset) => self.store::<i32>(*offset as usize),

            Instruction::I64Load(_, offset) => self.load::<i64>(*offset as usize),
            Instruction::I64Load8U(_, offset) => self.load_extend::<u8, i64>(*offset as usize),
            Instruction::I64Load16U(_, offset) => self.load_extend::<u16, i64>(*offset as usize),
            Instruction::I64Load32U(_, offset) => self.load_extend::<u32, i64>(*offset as usize),
            Instruction::I64Load8S(_, offset) => self.load_extend::<i8, i64>(*offset as usize),
            Instruction::I64Load16S(_, offset) => self.load_extend::<i16, i64>(*offset as usize),
            Instruction::I64Load32S(_, offset) => self.load_extend::<i32, i64>(*offset as usize),
            Instruction::I64Store(_, offset) => self.store::<i64>(*offset as usize),

            Instruction::F32Load(_, offset) => self.load::<f32>(*offset as usize),
            Instruction::F32Store(_, offset) => self.store::<f32>(*offset as usize),

            Instruction::F64Load(_, offset) => self.load::<f64>(*offset as usize),
            Instruction::F64Store(_, offset) => self.store::<f64>(*offset as usize),

            Instruction::GrowMemory(_) => {
                let grow_page: i32 = self.pop_as();
                let frame = self.stack.current_frame();
                let mem_addr = MemoryAddr(frame.module_index(), 0);
                let mem = self.store.memory_mut(mem_addr);
                let size = mem.page_size();
                match mem.grow(grow_page as usize) {
                    Ok(_) => {
                        self.stack.push_value(Value::I32(size as i32));
                    }
                    Err(err) => {
                        println!("[Debug] Failed to grow memory {:?}", err);
                        self.stack.push_value(Value::I32(-1));
                    }
                }
                Ok(ExecSuccess::Next)
            }
            Instruction::Block(ty) => {
                self.stack.push_label(Label::Block({
                    match ty {
                        BlockType::Value(_) => 1,
                        BlockType::NoResult => 0,
                    }
                }));
                Ok(ExecSuccess::Next)
            }
            Instruction::Loop(_) => {
                self.stack.push_label(Label::new_loop(self.pc.inst_index()));
                Ok(ExecSuccess::Next)
            }
            Instruction::If(ty) => {
                let val: i32 = self.pop_as();
                self.stack.push_label(Label::If(match ty {
                    BlockType::Value(_) => 1,
                    BlockType::NoResult => 0,
                }));
                if val == 0 {
                    let mut depth = 1;
                    loop {
                        let index = self.pc.inst_index().0 as usize;
                        match self.current_func_insts()[index] {
                            Instruction::End => depth -= 1,
                            Instruction::Block(_) => depth += 1,
                            Instruction::If(_) => depth += 1,
                            Instruction::Loop(_) => depth += 1,
                            Instruction::Else => {
                                if depth == 1 {
                                    self.pc.inc_inst_index();
                                    break;
                                }
                            }
                            _ => (),
                        }
                        if depth == 0 {
                            break;
                        }
                        self.pc.inc_inst_index();
                    }
                }
                Ok(ExecSuccess::Next)
            }
            Instruction::Else => self.branch(0),
            Instruction::BrIf(depth) => {
                let val = self.stack.pop_value();
                if val != Value::I32(0) {
                    self.branch(*depth)
                } else {
                    Ok(ExecSuccess::Next)
                }
            }
            Instruction::BrTable(ref payload) => {
                let val: i32 = self.pop_as();
                let val = val as usize;
                let depth = if val < payload.table.len() {
                    payload.table[val]
                } else {
                    payload.default
                };
                self.branch(depth)
            }
            Instruction::Br(depth) => self.branch(*depth),
            Instruction::Call(func_index) => {
                let frame = self.stack.current_frame();
                let addr = FuncAddr(frame.module_index(), *func_index as usize);
                self.invoke(addr)
            }
            Instruction::CallIndirect(type_index, _) => {
                let (ty, addr) = {
                    let frame = self.stack.current_frame();
                    let addr = TableAddr(frame.module_index(), 0);
                    let module = self.store.module(frame.module_index());
                    let ty = match module.get_type(*type_index as usize) {
                        parity_wasm::elements::Type::Function(ty) => ty,
                    };
                    (ty.clone(), addr)
                };
                let buf_index: i32 = self.pop_as();
                let table = self.store.table(addr);
                let buf_index = buf_index as usize;
                assert!(buf_index < table.buffer_len());
                let func_addr = match table.get_at(buf_index) {
                    Some(addr) => addr,
                    None => panic!(),
                };
                let func = self.store.func(func_addr);
                assert_eq!(*func.ty(), ty);
                self.invoke(func_addr)
            }
            Instruction::Return => self.do_return(),
            Instruction::End => {
                if self.stack.is_func_top_level() {
                    // When the end of a function is reached without a jump
                    let frame = self.stack.current_frame().clone();
                    let func = self.store.func(frame.func_addr);
                    println!("--- End of function {:?} ---", func.ty());
                    let arity = func.ty().return_type().map(|_| 1).unwrap_or(0);
                    let mut result = vec![];
                    for _ in 0..arity {
                        result.push(self.stack.pop_value());
                    }
                    println!("{:?}", self.stack);
                    self.stack.pop_label();
                    self.stack.pop_frame();
                    for v in result {
                        self.stack.push_value(v);
                    }
                    println!("--- End of finish process ---");
                    if let Some(ret_pc) = frame.ret_pc {
                        self.pc = ret_pc;
                        Ok(ExecSuccess::Next)
                    } else {
                        Ok(ExecSuccess::End)
                    }
                } else {
                    // When the end of a block is reached without a jump
                    let results = self.stack.pop_while(|v| match v {
                        StackValue::Value(_) => true,
                        _ => false,
                    });
                    self.stack.pop_label();
                    for v in results {
                        self.stack.push_value(*v.as_value().unwrap());
                    }
                    Ok(ExecSuccess::Next)
                }
            }
            Instruction::Nop => Ok(ExecSuccess::Next),
            _ => {
                debug_assert!(false, format!("{} not supported yet", inst));
                ExecResult::Err(ExecError::Panic(format!("{} not supported yet", inst)))
            }
        };
        if self.stack.is_over_top_level() {
            return Ok(ExecSuccess::End);
        } else {
            return result;
        }
    }

    fn pop_as<T: TryFrom<Value>>(&mut self) -> T {
        let value = self.stack.pop_value();
        match T::try_from(value) {
            Ok(val) => val,
            Err(_) => panic!(),
        }
    }

    fn branch(&mut self, depth: u32) -> ExecResult {
        let depth = depth as usize;
        let label = {
            let labels = self.stack.current_frame_labels();
            let labels_len = labels.len();
            assert!(depth + 1 <= labels_len);
            *labels[labels_len - depth - 1]
        };

        let arity = label.arity();

        let mut results = vec![];
        for _ in 0..arity {
            results.push(self.stack.pop_value());
        }

        for _ in 0..depth + 1 {
            self.stack.pop_while(|v| match v {
                StackValue::Value(_) => true,
                _ => false,
            });
            self.stack.pop_label();
        }

        for _ in 0..arity {
            self.stack.push_value(results.pop().unwrap());
        }

        // Jump to the continuation
        println!("> Jump to the continuation");
        match label {
            Label::Loop(loop_label) => self.pc.loop_jump(&loop_label),
            Label::Return(_) => {
                return self.do_return();
            }
            Label::If(_) | Label::Block(_) => {
                let mut depth = depth + 1;
                loop {
                    let index = self.pc.inst_index().0 as usize;
                    match self.current_func_insts()[index] {
                        Instruction::End => depth -= 1,
                        Instruction::Block(_) => depth += 1,
                        Instruction::If(_) => depth += 1,
                        Instruction::Loop(_) => depth += 1,
                        _ => (),
                    }
                    self.pc.inc_inst_index();
                    if depth == 0 {
                        break;
                    }
                }
            }
        }
        Ok(ExecSuccess::Next)
    }

    fn int_int_op<T: TryFrom<Value>, F: Fn(T, T) -> Value>(&mut self, f: F) -> ExecResult {
        let rhs = self.pop_as();
        let lhs = self.pop_as();
        self.stack.push_value(f(lhs, rhs));
        Ok(ExecSuccess::Next)
    }

    fn int_op<T: TryFrom<Value>, F: Fn(T) -> Value>(&mut self, f: F) -> ExecResult {
        let v: T = self.pop_as();
        self.stack.push_value(f(v));
        Ok(ExecSuccess::Next)
    }

    fn invoke(&mut self, addr: FuncAddr) -> ExecResult {
        let func = self.store.func(addr);
        let arity = func.ty().return_type().map(|_| 1).unwrap_or(0);
        println!("--- Start of Function {:?} ---", func.ty());

        println!("{:?}", self.stack);
        let mut args = Vec::new();
        for _ in func.ty().params() {
            args.push(self.stack.pop_value());
        }
        match func {
            FunctionInstance::Defined(defined) => {
                let pc = ProgramCounter::new(addr, InstIndex::zero());
                args.reverse();
                let frame = CallFrame::new_from_func(addr, &defined, args, Some(self.pc));
                self.stack.set_frame(frame);
                self.stack.push_label(Label::Return(arity));
                self.pc = pc;
                Ok(ExecSuccess::Next)
            }
            FunctionInstance::Host(host) => match &host.field_name()[..] {
                "print_i32" => {
                    BuiltinPrintI32::dispatch(&args);
                    Ok(ExecSuccess::Next)
                }
                _ => panic!(),
            },
        }
    }
    fn do_return(&mut self) -> ExecResult {
        let frame = self.stack.current_frame().clone();
        let func = self.store.func(frame.func_addr);
        println!("--- Function return {:?} ---", func.ty());
        let arity = func.ty().return_type().map(|_| 1).unwrap_or(0);
        let mut result = vec![];
        for _ in 0..arity {
            result.push(self.stack.pop_value());
        }
        self.stack.pop_while(|v| match v {
            StackValue::Activation(_) => false,
            _ => true,
        });
        self.stack.pop_frame();
        for v in result {
            self.stack.push_value(v);
        }

        if let Some(ret_pc) = frame.ret_pc {
            self.pc = ret_pc;
        }
        Ok(ExecSuccess::Next)
    }

    fn set_local(&mut self, index: usize) -> ExecResult {
        let value = self.stack.pop_value();
        self.stack.set_local(index, value);

        Ok(ExecSuccess::Next)
    }

    fn store<T: TryFrom<Value> + IntoLittleEndian>(&mut self, offset: usize) -> ExecResult {
        let val: T = self.pop_as();
        let raw_addr: i32 = self.pop_as();
        let raw_addr = raw_addr as usize;
        let addr: usize = raw_addr + offset;
        let frame = self.stack.current_frame();
        let mem_addr = MemoryAddr(frame.module_index(), 0);
        let memory = { self.store.memory(mem_addr) };
        let mem_len = memory.data_len();
        let elem_size = std::mem::size_of::<T>();
        if (addr + elem_size) > mem_len {
            panic!();
        }
        let mut buf: Vec<u8> = std::iter::repeat(0).take(elem_size).collect();
        val.into_le(&mut buf);
        self.store.memory_mut(mem_addr).initialize(addr, &buf);
        Ok(ExecSuccess::Next)
    }
    fn load<T>(&mut self, offset: usize) -> ExecResult
    where
        T: TryFrom<Value> + FromLittleEndian,
        T: Into<Value>,
    {
        let raw_addr: i32 = self.pop_as();
        let raw_addr = raw_addr as usize;
        let addr: usize = raw_addr + offset;

        let frame = self.stack.current_frame();
        let mem_addr = MemoryAddr(frame.module_index(), 0);
        let memory = { self.store.memory(mem_addr) };
        let mem_len = memory.data_len();
        let elem_size = std::mem::size_of::<T>();
        if (addr + elem_size) > mem_len {
            panic!();
        }
        let result: T = memory.load_as(addr);
        self.stack.push_value(result.into());
        Ok(ExecSuccess::Next)
    }

    fn load_extend<T: FromLittleEndian + ExtendInto<U>, U: Into<Value>>(
        &mut self,
        offset: usize,
    ) -> ExecResult {
        let raw_addr: i32 = self.pop_as();
        let raw_addr = raw_addr as usize;
        let addr: usize = raw_addr + offset;

        let frame = self.stack.current_frame();
        let mem_addr = MemoryAddr(frame.module_index(), 0);
        let memory = { self.store.memory(mem_addr) };
        let mem_len = memory.data_len();
        let elem_size = std::mem::size_of::<T>();
        if (addr + elem_size) > mem_len {
            panic!();
        }
        let result: T = memory.load_as(addr);
        let result = result.extend_into();
        self.stack.push_value(result.into());
        Ok(ExecSuccess::Next)
    }
}

pub fn eval_const_expr(init_expr: &InitExpr) -> Value {
    let inst = &init_expr.code()[0];
    match *inst {
        Instruction::I32Const(val) => Value::I32(val),
        Instruction::I64Const(val) => Value::I64(val),
        Instruction::F32Const(val) => Value::F32(f32::from_bits(val)),
        Instruction::F64Const(val) => Value::F64(f64::from_bits(val)),
        Instruction::GetGlobal(_) => panic!(),
        _ => panic!("Unsupported init_expr {}", inst),
    }
}
