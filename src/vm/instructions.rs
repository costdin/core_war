use super::numeric::Numeric;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum OpCode {
    Dat,
    Mov,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Jmp,
    Jmz,
    Jmn,
    Djn,
    Cmp,
    Slt,
    Spl,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Modifier {
    A,
    B,
    AB,
    BA,
    F,
    X,
    I,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Instruction<const CORE_SIZE: usize> {
    pub op: OpCode,
    pub modifier: Modifier,
    pub a_operand: Operand<CORE_SIZE>,
    pub b_operand: Operand<CORE_SIZE>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Operand<const CORE_SIZE: usize> {
    pub pointer: Numeric<CORE_SIZE>,
    pub mode: OperandMode,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum OperandMode {
    Immediate,
    Direct,
    Indirect,
    Decrement,
    Increment,
}
