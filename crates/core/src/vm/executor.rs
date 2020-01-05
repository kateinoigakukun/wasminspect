use super::address::{FuncAddr, GlobalAddr, MemoryAddr, TableAddr};
use super::func::*;
use super::global::*;
use super::host::*;
use super::memory;
use super::memory::MemoryInstance;
use super::module::*;
use super::stack;
use super::stack::{CallFrame, Label, ProgramCounter, Stack, StackValue};
use super::store::*;
use super::table;
use super::utils::*;
use super::value;
use super::value::{
    ExtendInto, FromLittleEndian, IntoLittleEndian, NativeValue, Value, F32, F64, I32, I64, U32,
    U64,
};
use parity_wasm::elements::{BlockType, FunctionType, InitExpr, Instruction, ValueType};

use std::ops::*;

#[derive(Debug)]
pub enum Trap {
    Unreachable,
    Memory(memory::Error),
    Stack(stack::Error),
    Table(table::Error),
    Value(value::Error),
    IndirectCallTypeMismatch(
        /* expected: */ FunctionType,
        /* actual: */ FunctionType,
    ),
    UnexpectedStackValueType(/* expected: */ ValueType, /* actual: */ ValueType),
    UndefinedFunc(FuncAddr),
}

impl std::fmt::Display for Trap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory(e) => write!(f, "{}", e),
            Self::Value(e) => write!(f, "{}", e),
            Self::Table(e) => write!(f, "{}", e),
            Self::Stack(e) => write!(f, "{}", e),
            Self::IndirectCallTypeMismatch(expected, actual) => write!(
                f,
                "indirect call type mismatch, expected {:?} but got {:?}",
                expected, actual
            ),
            Self::UndefinedFunc(addr) => write!(f, "uninitialized func at {:?}", addr),
            Self::Unreachable => write!(f, "unreachable"),
            _ => write!(f, "{:?}", self),
        }
    }
}

pub enum Signal {
    Next,
    End,
}

pub type ExecResult<T> = std::result::Result<T, Trap>;

#[derive(Debug)]
pub enum ReturnValError {
    TypeMismatchReturnValue(Value, ValueType),
    Stack(stack::Error),
    NoValue(ValueType),
}

pub type ReturnValResult = Result<Vec<Value>, ReturnValError>;

pub struct Executor<'a> {
    store: &'a mut Store,
    pc: ProgramCounter,
    stack: Stack,
}

impl<'a> Executor<'a> {
    pub fn new(
        initial_frame: CallFrame,
        initial_arity: usize,
        pc: ProgramCounter,
        store: &'a mut Store,
    ) -> Self {
        let mut stack = Stack::default();
        let _ = stack.set_frame(initial_frame);
        stack.push_label(Label::Return(initial_arity));
        Self { store, pc, stack }
    }

    pub fn pop_result(&mut self, return_ty: Vec<ValueType>) -> ReturnValResult {
        let mut results = vec![];
        for ty in return_ty {
            let val = self.stack.pop_value().map_err(ReturnValError::Stack)?;
            results.push(val);
            if val.value_type() != ty {
                return Err(ReturnValError::TypeMismatchReturnValue(val.clone(), ty));
            }
        }
        Ok(results)
    }

    pub fn current_func_insts(&self) -> ExecResult<&[Instruction]> {
        let addr = self.stack.current_func_addr().map_err(Trap::Stack)?;
        let func = self.store.func(addr).ok_or(Trap::UndefinedFunc(addr))?;
        Ok(&func.defined().unwrap().code().instructions())
    }

    pub fn execute_step(&mut self) -> ExecResult<Signal> {
        let func = self
            .store
            .func(self.pc.func_addr())
            .ok_or(Trap::UndefinedFunc(self.pc.func_addr()))?
            .defined()
            .unwrap();
        let module_index = func.module_index().clone();
        let inst = func.code().inst(self.pc.inst_index()).clone();
        return self.execute_inst(&inst, module_index);
    }

    fn execute_inst(
        &mut self,
        inst: &Instruction,
        module_index: ModuleIndex,
    ) -> ExecResult<Signal> {
        self.pc.inc_inst_index();
        // println!("{:?}", self.stack);
        // {
        //     let mut indent = String::new();
        //     for _ in 0..self
        //         .stack
        //         .current_frame_labels()
        //         .map_err(Trap::Stack)?
        //         .len()
        //     {
        //         indent.push_str("  ");
        //     }
        //     println!("{}{}", indent, inst.clone());
        // }
        let result = match inst {
            Instruction::Unreachable => Err(Trap::Unreachable),
            Instruction::Nop => Ok(Signal::Next),
            Instruction::Block(ty) => {
                self.stack.push_label(Label::Block({
                    match ty {
                        BlockType::Value(_) => 1,
                        BlockType::NoResult => 0,
                    }
                }));
                Ok(Signal::Next)
            }
            Instruction::Loop(_) => {
                let start_loop = InstIndex(self.pc.inst_index().0 - 1);
                self.stack.push_label(Label::new_loop(start_loop));
                Ok(Signal::Next)
            }
            Instruction::If(ty) => {
                let val: i32 = self.pop_as()?;
                self.stack.push_label(Label::If(match ty {
                    BlockType::Value(_) => 1,
                    BlockType::NoResult => 0,
                }));
                if val == 0 {
                    let mut depth = 1;
                    loop {
                        let index = self.pc.inst_index().0 as usize;
                        match self.current_func_insts()?[index] {
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
                Ok(Signal::Next)
            }
            Instruction::Else => self.branch(0),
            Instruction::End => {
                if self.stack.is_func_top_level().map_err(Trap::Stack)? {
                    // When the end of a function is reached without a jump
                    let frame = self.stack.current_frame().map_err(Trap::Stack)?.clone();
                    let func = self
                        .store
                        .func(frame.func_addr)
                        .ok_or(Trap::UndefinedFunc(frame.func_addr))?;
                    let arity = func.ty().return_type().map(|_| 1).unwrap_or(0);
                    let mut result = vec![];
                    for _ in 0..arity {
                        result.push(self.stack.pop_value().map_err(Trap::Stack)?);
                    }
                    self.stack.pop_label().map_err(Trap::Stack)?;
                    self.stack.pop_frame().map_err(Trap::Stack)?;
                    for v in result {
                        self.stack.push_value(v);
                    }
                    if let Some(ret_pc) = frame.ret_pc {
                        self.pc = ret_pc;
                        Ok(Signal::Next)
                    } else {
                        Ok(Signal::End)
                    }
                } else {
                    // When the end of a block is reached without a jump
                    let results = self.stack.pop_while(|v| match v {
                        StackValue::Value(_) => true,
                        _ => false,
                    });
                    self.stack.pop_label().map_err(Trap::Stack)?;
                    for v in results {
                        self.stack.push_value(v.as_value().map_err(Trap::Stack)?);
                    }
                    Ok(Signal::Next)
                }
            }
            Instruction::Br(depth) => self.branch(*depth),
            Instruction::BrIf(depth) => {
                let val = self.stack.pop_value().map_err(Trap::Stack)?;
                if val != Value::I32(0) {
                    self.branch(*depth)
                } else {
                    Ok(Signal::Next)
                }
            }
            Instruction::BrTable(ref payload) => {
                let val: i32 = self.pop_as()?;
                let val = val as usize;
                let depth = if val < payload.table.len() {
                    payload.table[val]
                } else {
                    payload.default
                };
                self.branch(depth)
            }
            Instruction::Return => self.do_return(),
            Instruction::Call(func_index) => {
                let frame = self.stack.current_frame().map_err(Trap::Stack)?;
                let addr = FuncAddr(frame.module_index(), *func_index as usize);
                self.invoke(addr)
            }
            Instruction::CallIndirect(type_index, _) => {
                let (ty, addr) = {
                    let frame = self.stack.current_frame().map_err(Trap::Stack)?;
                    let addr = TableAddr(frame.module_index(), 0);
                    let module = self.store.module(frame.module_index()).defined().unwrap();
                    let ty = match module.get_type(*type_index as usize) {
                        parity_wasm::elements::Type::Function(ty) => ty,
                    };
                    (ty.clone(), addr)
                };
                let buf_index: i32 = self.pop_as()?;
                let table = self.store.table(addr);
                let buf_index = buf_index as usize;
                let func_addr = table
                    .borrow()
                    .get_at(buf_index, self.store)
                    .map_err(Trap::Table)?;
                let func = self
                    .store
                    .func(func_addr)
                    .ok_or(Trap::UndefinedFunc(func_addr))?;
                if *func.ty() == ty {
                    self.invoke(func_addr)
                } else {
                    Err(Trap::IndirectCallTypeMismatch(ty, func.ty().clone()))
                }
            }
            Instruction::Drop => {
                self.stack.pop_value().map_err(Trap::Stack)?;
                Ok(Signal::Next)
            }
            Instruction::Select => {
                let cond: i32 = self.pop_as()?;
                let val2 = self.stack.pop_value().map_err(Trap::Stack)?;
                let val1 = self.stack.pop_value().map_err(Trap::Stack)?;
                if cond != 0 {
                    self.stack.push_value(val1);
                } else {
                    self.stack.push_value(val2);
                }
                Ok(Signal::Next)
            }
            Instruction::GetLocal(index) => {
                let value = self
                    .stack
                    .current_frame()
                    .map_err(Trap::Stack)?
                    .local(*index as usize);
                self.stack.push_value(value);
                Ok(Signal::Next)
            }
            Instruction::SetLocal(index) => self.set_local(*index as usize),
            Instruction::TeeLocal(index) => {
                let val = self.stack.pop_value().map_err(Trap::Stack)?;
                self.stack.push_value(val);
                self.stack.push_value(val);
                self.set_local(*index as usize)
            }
            Instruction::GetGlobal(index) => {
                let addr = GlobalAddr(module_index, *index as usize);
                let global = self.store.global(addr);
                self.stack.push_value(global.borrow().value(self.store));
                Ok(Signal::Next)
            }
            Instruction::SetGlobal(index) => {
                let addr = GlobalAddr(module_index, *index as usize);
                let value = self.stack.pop_value().map_err(Trap::Stack)?;
                let global = resolve_global_instance(addr, self.store);
                global.borrow_mut().set_value(value);
                Ok(Signal::Next)
            }

            Instruction::I32Load(_, offset) => self.load::<i32>(*offset as usize),
            Instruction::I64Load(_, offset) => self.load::<i64>(*offset as usize),
            Instruction::F32Load(_, offset) => self.load::<f32>(*offset as usize),
            Instruction::F64Load(_, offset) => self.load::<f64>(*offset as usize),

            Instruction::I32Load8S(_, offset) => self.load_extend::<i8, i32>(*offset as usize),
            Instruction::I32Load8U(_, offset) => self.load_extend::<u8, i32>(*offset as usize),
            Instruction::I32Load16S(_, offset) => self.load_extend::<i16, i32>(*offset as usize),
            Instruction::I32Load16U(_, offset) => self.load_extend::<u16, i32>(*offset as usize),

            Instruction::I64Load8S(_, offset) => self.load_extend::<i8, i64>(*offset as usize),
            Instruction::I64Load8U(_, offset) => self.load_extend::<u8, i64>(*offset as usize),
            Instruction::I64Load16S(_, offset) => self.load_extend::<i16, i64>(*offset as usize),
            Instruction::I64Load16U(_, offset) => self.load_extend::<u16, i64>(*offset as usize),
            Instruction::I64Load32S(_, offset) => self.load_extend::<i32, i64>(*offset as usize),
            Instruction::I64Load32U(_, offset) => self.load_extend::<u32, i64>(*offset as usize),

            Instruction::I32Store(_, offset) => self.store::<i32>(*offset as usize),
            Instruction::I64Store(_, offset) => self.store::<i64>(*offset as usize),
            Instruction::F32Store(_, offset) => self.store::<f32>(*offset as usize),
            Instruction::F64Store(_, offset) => self.store::<f64>(*offset as usize),

            Instruction::I32Store8(_, offset) => self.store_with_width::<i32>(*offset as usize, 1),
            Instruction::I32Store16(_, offset) => self.store_with_width::<i32>(*offset as usize, 2),
            Instruction::I64Store8(_, offset) => self.store_with_width::<i64>(*offset as usize, 1),
            Instruction::I64Store16(_, offset) => self.store_with_width::<i64>(*offset as usize, 2),
            Instruction::I64Store32(_, offset) => self.store_with_width::<i64>(*offset as usize, 4),

            Instruction::CurrentMemory(_) => {
                self.stack.push_value(Value::I32(
                    self.memory()?.borrow().page_count(self.store) as i32
                ));
                Ok(Signal::Next)
            }
            Instruction::GrowMemory(_) => {
                let grow_page: i32 = self.pop_as()?;
                let mem = self.memory()?;
                let size = mem.borrow().page_count(self.store);
                match mem.borrow_mut().grow(grow_page as usize, self.store) {
                    Ok(_) => {
                        self.stack.push_value(Value::I32(size as i32));
                    }
                    Err(err) => {
                        println!("[Debug] Failed to grow memory {:?}", err);
                        self.stack.push_value(Value::I32(-1));
                    }
                }
                Ok(Signal::Next)
            }

            Instruction::I32Const(val) => {
                self.stack.push_value(Value::I32(*val));
                Ok(Signal::Next)
            }
            Instruction::I64Const(val) => {
                self.stack.push_value(Value::I64(*val));
                Ok(Signal::Next)
            }
            Instruction::F32Const(val) => {
                self.stack.push_value(Value::F32(f32::from_bits(*val)));
                Ok(Signal::Next)
            }
            Instruction::F64Const(val) => {
                self.stack.push_value(Value::F64(f64::from_bits(*val)));
                Ok(Signal::Next)
            }

            Instruction::I32Eqz => self.testop::<i32, _>(|v| v == 0),
            Instruction::I32Eq => self.relop(|a: i32, b: i32| a == b),
            Instruction::I32Ne => self.relop(|a: i32, b: i32| a != b),
            Instruction::I32LtS => self.relop(|a: i32, b: i32| a < b),
            Instruction::I32LtU => self.relop::<u32, _>(|a, b| a < b),
            Instruction::I32GtS => self.relop(|a: i32, b: i32| a > b),
            Instruction::I32GtU => self.relop::<u32, _>(|a, b| a > b),
            Instruction::I32LeS => self.relop(|a: i32, b: i32| a <= b),
            Instruction::I32LeU => self.relop::<u32, _>(|a, b| a <= b),
            Instruction::I32GeS => self.relop(|a: i32, b: i32| a >= b),
            Instruction::I32GeU => self.relop::<u32, _>(|a, b| a >= b),

            Instruction::I64Eqz => self.testop::<i64, _>(|v| v == 0),
            Instruction::I64Eq => self.relop(|a: i64, b: i64| a == b),
            Instruction::I64Ne => self.relop(|a: i64, b: i64| a != b),
            Instruction::I64LtS => self.relop(|a: i64, b: i64| a < b),
            Instruction::I64LtU => self.relop::<u64, _>(|a, b| a < b),
            Instruction::I64GtS => self.relop(|a: i64, b: i64| a > b),
            Instruction::I64GtU => self.relop::<u64, _>(|a, b| a > b),
            Instruction::I64LeS => self.relop(|a: i64, b: i64| a <= b),
            Instruction::I64LeU => self.relop::<u64, _>(|a, b| a <= b),
            Instruction::I64GeS => self.relop(|a: i64, b: i64| a >= b),
            Instruction::I64GeU => self.relop::<u64, _>(|a, b| a >= b),

            Instruction::F32Eq => self.relop::<f32, _>(|a, b| a == b),
            Instruction::F32Ne => self.relop::<f32, _>(|a, b| a != b),
            Instruction::F32Lt => self.relop::<f32, _>(|a, b| a < b),
            Instruction::F32Gt => self.relop::<f32, _>(|a, b| a > b),
            Instruction::F32Le => self.relop::<f32, _>(|a, b| a <= b),
            Instruction::F32Ge => self.relop::<f32, _>(|a, b| a >= b),

            Instruction::F64Eq => self.relop(|a: f64, b: f64| a == b),
            Instruction::F64Ne => self.relop(|a: f64, b: f64| a != b),
            Instruction::F64Lt => self.relop(|a: f64, b: f64| a < b),
            Instruction::F64Gt => self.relop(|a: f64, b: f64| a > b),
            Instruction::F64Le => self.relop(|a: f64, b: f64| a <= b),
            Instruction::F64Ge => self.relop(|a: f64, b: f64| a >= b),

            Instruction::I32Clz => self.unop(|v: i32| v.leading_zeros() as i32),
            Instruction::I32Ctz => self.unop(|v: i32| v.trailing_zeros() as i32),
            Instruction::I32Popcnt => self.unop(|v: i32| v.count_ones() as i32),
            Instruction::I32Add => self.binop(|a: u32, b: u32| a.wrapping_add(b)),
            Instruction::I32Sub => self.binop(|a: i32, b: i32| a.wrapping_sub(b)),
            Instruction::I32Mul => self.binop(|a: i32, b: i32| a.wrapping_mul(b)),
            Instruction::I32DivS => self.try_binop(|a: i32, b: i32| I32::try_wrapping_div(a, b)),
            Instruction::I32DivU => self.try_binop(|a: u32, b: u32| U32::try_wrapping_div(a, b)),
            Instruction::I32RemS => self.try_binop(|a: i32, b: i32| I32::try_wrapping_rem(a, b)),
            Instruction::I32RemU => self.try_binop(|a: u32, b: u32| U32::try_wrapping_rem(a, b)),
            Instruction::I32And => self.binop(|a: i32, b: i32| a.bitand(b)),
            Instruction::I32Or => self.binop(|a: i32, b: i32| a.bitor(b)),
            Instruction::I32Xor => self.binop(|a: i32, b: i32| a.bitxor(b)),
            Instruction::I32Shl => self.binop(|a: u32, b: u32| a.wrapping_shl(b)),
            Instruction::I32ShrS => self.binop(|a: i32, b: i32| a.wrapping_shr(b as u32)),
            Instruction::I32ShrU => self.binop(|a: u32, b: u32| a.wrapping_shr(b)),
            Instruction::I32Rotl => self.binop(|a: i32, b: i32| a.rotate_left(b as u32)),
            Instruction::I32Rotr => self.binop(|a: i32, b: i32| a.rotate_right(b as u32)),

            Instruction::I64Clz => self.unop(|v: i64| v.leading_zeros() as i64),
            Instruction::I64Ctz => self.unop(|v: i64| v.trailing_zeros() as i64),
            Instruction::I64Popcnt => self.unop(|v: i64| v.count_ones() as i64),
            Instruction::I64Add => self.binop(|a: i64, b: i64| a.wrapping_add(b)),
            Instruction::I64Sub => self.binop(|a: i64, b: i64| a.wrapping_sub(b)),
            Instruction::I64Mul => self.binop(|a: i64, b: i64| a.wrapping_mul(b)),
            Instruction::I64DivS => self.try_binop(|a: i64, b: i64| I64::try_wrapping_div(a, b)),
            Instruction::I64DivU => self.try_binop(|a: u64, b: u64| U64::try_wrapping_div(a, b)),
            Instruction::I64RemS => self.try_binop(|a: i64, b: i64| I64::try_wrapping_rem(a, b)),
            Instruction::I64RemU => self.try_binop(|a: u64, b: u64| U64::try_wrapping_rem(a, b)),
            Instruction::I64And => self.binop(|a: i64, b: i64| a.bitand(b)),
            Instruction::I64Or => self.binop(|a: i64, b: i64| a.bitor(b)),
            Instruction::I64Xor => self.binop(|a: i64, b: i64| a.bitxor(b)),
            Instruction::I64Shl => self.binop(|a: u64, b: u64| a.wrapping_shl(b as u32)),
            Instruction::I64ShrS => self.binop(|a: i64, b: i64| a.wrapping_shr(b as u32)),
            Instruction::I64ShrU => self.binop(|a: u64, b: u64| a.wrapping_shr(b as u32)),
            Instruction::I64Rotl => self.binop(|a: i64, b: i64| a.rotate_left(b as u32)),
            Instruction::I64Rotr => self.binop(|a: i64, b: i64| a.rotate_right(b as u32)),

            Instruction::F32Abs => self.unop(|v: f32| v.abs()),
            Instruction::F32Neg => self.unop(|v: f32| -v),
            Instruction::F32Ceil => self.unop(|v: f32| v.ceil()),
            Instruction::F32Floor => self.unop(|v: f32| v.floor()),
            Instruction::F32Trunc => self.unop(|v: f32| v.trunc()),
            Instruction::F32Nearest => self.unop(|v: f32| F32::nearest(v)),
            Instruction::F32Sqrt => self.unop(|v: f32| Value::F32(v.sqrt())),
            Instruction::F32Add => self.binop(|a: f32, b: f32| Value::F32(a + b)),
            Instruction::F32Sub => self.binop(|a: f32, b: f32| Value::F32(a - b)),
            Instruction::F32Mul => self.binop(|a: f32, b: f32| Value::F32(a * b)),
            Instruction::F32Div => self.binop(|a: f32, b: f32| Value::F32(a / b)),
            Instruction::F32Min => self.binop(|a: f32, b: f32| F32::min(a, b)),
            Instruction::F32Max => self.binop(|a: f32, b: f32| F32::max(a, b)),
            Instruction::F32Copysign => {
                self.binop(|a: f32, b: f32| Value::F32(F32::copysign(a, b)))
            }

            Instruction::F64Abs => self.unop(|v: f64| Value::F64(v.abs())),
            Instruction::F64Neg => self.unop(|v: f64| Value::F64(-v)),
            Instruction::F64Ceil => self.unop(|v: f64| Value::F64(v.ceil())),
            Instruction::F64Floor => self.unop(|v: f64| Value::F64(v.floor())),
            Instruction::F64Trunc => self.unop(|v: f64| Value::F64(v.trunc())),
            Instruction::F64Nearest => self.unop(|v: f64| F64::nearest(v)),
            Instruction::F64Sqrt => self.unop(|v: f64| Value::F64(v.sqrt())),
            Instruction::F64Add => self.binop(|a: f64, b: f64| Value::F64(a + b)),
            Instruction::F64Sub => self.binop(|a: f64, b: f64| Value::F64(a - b)),
            Instruction::F64Mul => self.binop(|a: f64, b: f64| Value::F64(a * b)),
            Instruction::F64Div => self.binop(|a: f64, b: f64| Value::F64(a / b)),
            Instruction::F64Min => self.binop(|a: f64, b: f64| F64::min(a, b)),
            Instruction::F64Max => self.binop(|a: f64, b: f64| F64::max(a, b)),
            Instruction::F64Copysign => {
                self.binop(|a: f64, b: f64| Value::F64(F64::copysign(a, b)))
            }

            Instruction::I32WrapI64 => self.unop(|v: i64| Value::I32(v as i32)),
            Instruction::I32TruncSF32 => self.try_unop(|v: f32| F32::trunc_to_i32(v)),
            Instruction::I32TruncUF32 => self.try_unop(|v: f32| F32::trunc_to_u32(v)),
            Instruction::I32TruncSF64 => self.try_unop(|v: f64| F64::trunc_to_i32(v)),
            Instruction::I32TruncUF64 => self.try_unop(|v: f64| F64::trunc_to_u32(v)),
            Instruction::I64ExtendSI32 => self.unop(|v: i32| Value::from(v as u64)),
            Instruction::I64ExtendUI32 => self.unop(|v: u32| Value::from(v as u64)),
            Instruction::I64TruncSF32 => self.try_unop(|x: f32| F32::trunc_to_i64(x)),
            Instruction::I64TruncUF32 => self.try_unop(|x: f32| F32::trunc_to_u64(x)),
            Instruction::I64TruncSF64 => self.try_unop(|x: f64| F64::trunc_to_i64(x)),
            Instruction::I64TruncUF64 => self.try_unop(|x: f64| F64::trunc_to_u64(x)),
            Instruction::F32ConvertSI32 => self.unop(|x: u32| x as i32 as f32),
            Instruction::F32ConvertUI32 => self.unop(|x: u32| x as f32),
            Instruction::F32ConvertSI64 => self.unop(|x: u64| x as i64 as f32),
            Instruction::F32ConvertUI64 => self.unop(|x: u64| x as f32),
            Instruction::F32DemoteF64 => self.unop(|x: f64| x as f32),
            Instruction::F64ConvertSI32 => self.unop(|x: u32| f64::from(x as i32)),
            Instruction::F64ConvertUI32 => self.unop(|x: u32| f64::from(x)),
            Instruction::F64ConvertSI64 => self.unop(|x: u64| x as i64 as f64),
            Instruction::F64ConvertUI64 => self.unop(|x: u64| x as f64),
            Instruction::F64PromoteF32 => self.unop(|x: f32| f64::from(x)),

            Instruction::I32ReinterpretF32 => self.unop(|v: f32| v.to_bits() as i32),
            Instruction::I64ReinterpretF64 => self.unop(|v: f64| v.to_bits() as i64),
            Instruction::F32ReinterpretI32 => self.unop(f32::from_bits),
            Instruction::F64ReinterpretI64 => self.unop(f64::from_bits),
        };
        if self.stack.is_over_top_level() {
            return Ok(Signal::End);
        } else {
            return result;
        }
    }

    fn pop_as<T: NativeValue>(&mut self) -> ExecResult<T> {
        let value = self.stack.pop_value().map_err(Trap::Stack)?;
        T::from_value(value).ok_or(Trap::UnexpectedStackValueType(
            /* expected: */ T::value_type(),
            /* actual:   */ value.value_type(),
        ))
    }

    fn branch(&mut self, depth: u32) -> ExecResult<Signal> {
        let depth = depth as usize;
        let label = {
            let labels = self.stack.current_frame_labels().map_err(Trap::Stack)?;
            let labels_len = labels.len();
            assert!(depth + 1 <= labels_len);
            *labels[labels_len - depth - 1]
        };

        let arity = label.arity();

        let mut results = vec![];
        for _ in 0..arity {
            results.push(self.stack.pop_value().map_err(Trap::Stack)?);
        }

        for _ in 0..depth + 1 {
            self.stack.pop_while(|v| match v {
                StackValue::Value(_) => true,
                _ => false,
            });
            self.stack.pop_label().map_err(Trap::Stack)?;
        }

        for _ in 0..arity {
            self.stack.push_value(results.pop().unwrap());
        }

        // Jump to the continuation
        match label {
            Label::Loop(loop_label) => self.pc.loop_jump(&loop_label),
            Label::Return(_) => {
                return self.do_return();
            }
            Label::If(_) | Label::Block(_) => {
                let mut depth = depth + 1;
                loop {
                    let index = self.pc.inst_index().0 as usize;
                    match self.current_func_insts()?[index] {
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
        Ok(Signal::Next)
    }

    fn testop<T: NativeValue, F: Fn(T) -> bool>(&mut self, f: F) -> ExecResult<Signal> {
        self.unop(|a| Value::I32(if f(a) { 1 } else { 0 }))
    }

    fn relop<T: NativeValue, F: Fn(T, T) -> bool>(&mut self, f: F) -> ExecResult<Signal> {
        self.binop(|a: T, b: T| Value::I32(if f(a, b) { 1 } else { 0 }))
    }

    fn try_binop<T: NativeValue, To: Into<Value>, F: Fn(T, T) -> Result<To, value::Error>>(
        &mut self,
        f: F,
    ) -> ExecResult<Signal> {
        let rhs = self.pop_as()?;
        let lhs = self.pop_as()?;
        self.stack
            .push_value(f(lhs, rhs).map(|v| v.into()).map_err(Trap::Value)?);
        Ok(Signal::Next)
    }

    fn binop<T: NativeValue, To: Into<Value>, F: Fn(T, T) -> To>(
        &mut self,
        f: F,
    ) -> ExecResult<Signal> {
        let rhs = self.pop_as()?;
        let lhs = self.pop_as()?;
        self.stack.push_value(f(lhs, rhs).into());
        Ok(Signal::Next)
    }

    fn try_unop<From: NativeValue, To: Into<Value>, F: Fn(From) -> Result<To, value::Error>>(
        &mut self,
        f: F,
    ) -> ExecResult<Signal> {
        let v: From = self.pop_as()?;
        self.stack
            .push_value(f(v).map(|v| v.into()).map_err(Trap::Value)?);
        Ok(Signal::Next)
    }

    fn unop<From: NativeValue, To: Into<Value>, F: Fn(From) -> To>(
        &mut self,
        f: F,
    ) -> ExecResult<Signal> {
        let v: From = self.pop_as()?;
        self.stack.push_value(f(v).into());
        Ok(Signal::Next)
    }

    fn invoke(&mut self, addr: FuncAddr) -> ExecResult<Signal> {
        let func_ty = self.store.func_ty(addr);

        let mut args = Vec::new();
        for _ in func_ty.params() {
            args.push(self.stack.pop_value().map_err(Trap::Stack)?);
        }
        args.reverse();

        let func = self.store.func(addr).ok_or(Trap::UndefinedFunc(addr))?;
        let arity = func.ty().return_type().map(|_| 1).unwrap_or(0);
        let result = {
            resolve_func_addr(addr, self.store)?.clone()
        };
        match result {
            Either::Left((addr, func)) => {
                let pc = ProgramCounter::new(addr, InstIndex::zero());
                let frame = CallFrame::new_from_func(addr, &func, args, Some(self.pc));
                self.stack.set_frame(frame).map_err(Trap::Stack)?;
                self.stack.push_label(Label::Return(arity));
                self.pc = pc;
                Ok(Signal::Next)
            }
            Either::Right(host_func_body) => {
                let mut result = Vec::new();
                host_func_body.call(&args, &mut result, &mut self.store, addr.0)?;
                assert_eq!(result.len(), arity);
                for v in result {
                    self.stack.push_value(v);
                }
                Ok(Signal::Next)
            }
        }
    }
    fn do_return(&mut self) -> ExecResult<Signal> {
        let frame = self.stack.current_frame().map_err(Trap::Stack)?.clone();
        let func = self
            .store
            .func(frame.func_addr)
            .ok_or(Trap::UndefinedFunc(frame.func_addr))?;
        let arity = func.ty().return_type().map(|_| 1).unwrap_or(0);
        let mut result = vec![];
        for _ in 0..arity {
            result.push(self.stack.pop_value().map_err(Trap::Stack)?);
        }
        self.stack.pop_while(|v| match v {
            StackValue::Activation(_) => false,
            _ => true,
        });
        self.stack.pop_frame().map_err(Trap::Stack)?;
        for v in result {
            self.stack.push_value(v);
        }

        if let Some(ret_pc) = frame.ret_pc {
            self.pc = ret_pc;
        }
        Ok(Signal::Next)
    }

    fn set_local(&mut self, index: usize) -> ExecResult<Signal> {
        let value = self.stack.pop_value().map_err(Trap::Stack)?;
        self.stack.set_local(index, value).map_err(Trap::Stack)?;

        Ok(Signal::Next)
    }

    fn memory(&self) -> ExecResult<std::rc::Rc<std::cell::RefCell<MemoryInstance>>> {
        let frame = self.stack.current_frame().map_err(Trap::Stack)?;
        let mem_addr = MemoryAddr(frame.module_index(), 0);
        Ok(self.store.memory(mem_addr))
    }

    fn store<T: NativeValue + IntoLittleEndian>(&mut self, offset: usize) -> ExecResult<Signal> {
        let val: T = self.pop_as()?;
        let base_addr: i32 = self.pop_as()?;
        let base_addr = base_addr as usize;
        let addr = base_addr + offset;
        let mut buf: Vec<u8> = std::iter::repeat(0)
            .take(std::mem::size_of::<T>())
            .collect();
        val.into_le(&mut buf);
        self.memory()?
            .borrow_mut()
            .store(addr, &buf, self.store)
            .map_err(Trap::Memory)?;
        Ok(Signal::Next)
    }

    fn store_with_width<T: NativeValue + IntoLittleEndian>(
        &mut self,
        offset: usize,
        width: usize,
    ) -> ExecResult<Signal> {
        let val: T = self.pop_as()?;
        let base_addr: i32 = self.pop_as()?;
        let base_addr = base_addr as usize;
        let addr: usize = base_addr + offset;
        let mut buf: Vec<u8> = std::iter::repeat(0)
            .take(std::mem::size_of::<T>())
            .collect();
        val.into_le(&mut buf);
        let buf: Vec<u8> = buf.into_iter().take(width).collect();
        self.memory()?
            .borrow_mut()
            .store(addr, &buf, self.store)
            .map_err(Trap::Memory)?;
        Ok(Signal::Next)
    }

    fn load<T>(&mut self, offset: usize) -> ExecResult<Signal>
    where
        T: NativeValue + FromLittleEndian,
        T: Into<Value>,
    {
        let base_addr: i32 = self.pop_as()?;
        let base_addr = base_addr as usize;
        let addr: usize = base_addr + offset;

        let result: T = self
            .memory()?
            .borrow_mut()
            .load_as(addr, self.store)
            .map_err(Trap::Memory)?;
        self.stack.push_value(result.into());
        Ok(Signal::Next)
    }

    fn load_extend<T: FromLittleEndian + ExtendInto<U>, U: Into<Value>>(
        &mut self,
        offset: usize,
    ) -> ExecResult<Signal> {
        let base_addr: i32 = self.pop_as()?;
        let base_addr = base_addr as usize;
        let addr: usize = base_addr + offset;

        let result: T = self
            .memory()?
            .borrow_mut()
            .load_as(addr, self.store)
            .map_err(Trap::Memory)?;
        let result = result.extend_into();
        self.stack.push_value(result.into());
        Ok(Signal::Next)
    }
}

pub fn eval_const_expr(init_expr: &InitExpr, store: &Store, module_index: ModuleIndex) -> Value {
    let inst = &init_expr.code()[0];
    match *inst {
        Instruction::I32Const(val) => Value::I32(val),
        Instruction::I64Const(val) => Value::I64(val),
        Instruction::F32Const(val) => Value::F32(f32::from_bits(val)),
        Instruction::F64Const(val) => Value::F64(f64::from_bits(val)),
        Instruction::GetGlobal(index) => {
            let addr = GlobalAddr(module_index, index as usize);
            store.global(addr).borrow().value(store)
        }
        _ => panic!("Unsupported init_expr {}", inst),
    }
}

pub enum WasmError {
    ExecutionError(Trap),
    EntryFunctionNotFound(String),
    ReturnValueError(ReturnValError),
    HostExecutionError,
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmError::ExecutionError(err) => write!(f, "Failed to execute: {}", err),
            WasmError::EntryFunctionNotFound(func_name) => {
                write!(f, "Entry function \"{}\" not found", func_name)
            }
            WasmError::ReturnValueError(err) => {
                write!(f, "Failed to get returned value: {:?}", err)
            }
            WasmError::HostExecutionError => write!(f, "Failed to execute host func"),
        }
    }
}

pub fn resolve_func_addr(
    addr: FuncAddr,
    store: &Store,
) -> ExecResult<Either<(FuncAddr, &DefinedFunctionInstance), &HostFuncBody>> {
    let func = store.func(addr).ok_or(Trap::UndefinedFunc(addr))?;
    match func {
        FunctionInstance::Defined(defined) => Ok(Either::Left((addr, defined))),
        FunctionInstance::External(func) => {
            let module = store.module_by_name(func.module_name().clone());
            match module {
                ModuleInstance::Host(host_module) => {
                    let func = host_module
                        .func_by_name(func.field_name().clone())
                        .ok()
                        .unwrap()
                        .unwrap();
                    return Ok(Either::Right(func));
                }
                ModuleInstance::Defined(defined_module) => {
                    let addr = defined_module
                        .exported_func(func.field_name().clone())
                        .ok()
                        .unwrap()
                        .unwrap();
                    return resolve_func_addr(addr, store);
                }
            }
        }
    }
}

pub fn invoke_func(
    func_addr: FuncAddr,
    arguments: Vec<Value>,
    store: &mut Store,
) -> Result<Vec<Value>, WasmError> {
    match resolve_func_addr(func_addr, &store).map_err(WasmError::ExecutionError)? {
        Either::Right(host_func_body) => {
            let mut results = Vec::new();
            match host_func_body.call(&arguments, &mut results, store, func_addr.0) {
                Ok(_) => Ok(results),
                Err(_) => Err(WasmError::HostExecutionError),
            }
        }
        Either::Left((func_addr, func)) => {
            let (frame, ret_types) = {
                let ret_types = func.ty().return_type().map(|ty| vec![ty]).unwrap_or(vec![]);
                let frame = CallFrame::new_from_func(func_addr, func, arguments, None);
                (frame, ret_types)
            };
            let pc = ProgramCounter::new(func_addr, InstIndex::zero());
            let mut executor = Executor::new(frame, ret_types.len(), pc, store);
            loop {
                let result = executor.execute_step();
                match result {
                    Ok(Signal::Next) => continue,
                    Ok(Signal::End) => match executor.pop_result(ret_types) {
                        Ok(values) => return Ok(values),
                        Err(err) => return Err(WasmError::ReturnValueError(err)),
                    },
                    Err(err) => return Err(WasmError::ExecutionError(err)),
                }
            }
        }
    }
}
