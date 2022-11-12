use super::event::{EventType, Observable, Observer, VmEvent};
use super::instructions::*;
use super::numeric::Numeric;
use std::collections::VecDeque;

pub struct WarriorDefinition<const CORE_SIZE: usize> {
    pub name: String,
    pub ops: Vec<Instruction<CORE_SIZE>>,
}

impl<const CORE_SIZE: usize> WarriorDefinition<CORE_SIZE> {
    pub fn new(name: String, ops: Vec<Instruction<CORE_SIZE>>) -> WarriorDefinition<CORE_SIZE> {
        WarriorDefinition { name, ops }
    }
}

struct WarriorQueue<const CORE_SIZE: usize, const QUEUE_SIZE: usize> {
    warrior_id: usize,
    instruction_queue: VecDeque<Numeric<CORE_SIZE>>,
}

pub struct Vm<const CORE_SIZE: usize, const QUEUE_SIZE: usize> {
    core: Box<[Instruction<CORE_SIZE>; CORE_SIZE]>,
    warriors_definitions: Vec<WarriorDefinition<CORE_SIZE>>,
    warriors_queues: Vec<WarriorQueue<CORE_SIZE, QUEUE_SIZE>>,
    observers: Vec<Box<dyn Observer<VmEvent>>>,
    pub round: u128,
    next_warrior_id: usize,
}

impl<const CORE_SIZE: usize, const QUEUE_SIZE: usize> Observable<VmEvent>
    for Vm<CORE_SIZE, QUEUE_SIZE>
{
    fn register(&mut self, observer: Box<dyn Observer<VmEvent>>) {
        self.observers.push(observer);
    }
}

impl<const CORE_SIZE: usize, const QUEUE_SIZE: usize> Vm<CORE_SIZE, QUEUE_SIZE> {
    pub fn new(
        warriors_definitions: Vec<WarriorDefinition<CORE_SIZE>>,
    ) -> Result<Vm<CORE_SIZE, QUEUE_SIZE>, String> {
        if warriors_definitions.len() > 50 || warriors_definitions.len() < 2 {
            return Err("".to_string());
        }

        let mut core = Box::new(
            [Instruction {
                op: OpCode::Dat,
                modifier: Modifier::A,
                a_operand: Operand {
                    pointer: 0.into(),
                    mode: OperandMode::Direct,
                },
                b_operand: Operand {
                    pointer: 0.into(),
                    mode: OperandMode::Direct,
                },
            }; CORE_SIZE],
        );
        let mut warriors_alive = Vec::new();
        let mut instruction_pointer = 0;

        for (warrior_id, warrior_definition) in warriors_definitions.iter().enumerate() {
            for (ix, op) in warrior_definition.ops.iter().enumerate() {
                core[instruction_pointer + ix] = op.clone();
            }

            let mut instruction_queue = VecDeque::new();
            instruction_queue.push_back(Numeric::new(instruction_pointer));
            warriors_alive.push(WarriorQueue {
                warrior_id,
                instruction_queue,
            });

            instruction_pointer += CORE_SIZE / warriors_definitions.len();
        }

        Ok(Vm::<CORE_SIZE, QUEUE_SIZE> {
            core,
            warriors_definitions,
            warriors_queues: warriors_alive,
            observers: Vec::new(),
            round: 0,
            next_warrior_id: 0,
        })
    }

    pub fn notify_observers(&self, event: VmEvent) {
        for obs in self.observers.iter() {
            obs.notify(event.clone());
        }
    }

    pub fn play(&mut self, tick_count: i32) -> Option<&WarriorDefinition<CORE_SIZE>> {
        let mut ticks_played = 0;
        while self.warriors_queues.len() > 1 && ticks_played < tick_count {
            if let Some(instruction_pointer) = self.warriors_queues[self.next_warrior_id]
                .instruction_queue
                .pop_front()
            {
                ticks_played += 1;
                let instruction = self.core[instruction_pointer.value].clone();

                for new_ix in self.execute(instruction, instruction_pointer, self.next_warrior_id) {
                    self.notify_observers(VmEvent {
                        event_type: EventType::Jump,
                        moved_from: Some(instruction_pointer.value),
                        offset: Some(new_ix.value),
                        warrior_id: self.warriors_queues[self.next_warrior_id].warrior_id,
                        round: self.round,
                    });

                    self.warriors_queues[self.next_warrior_id]
                        .instruction_queue
                        .push_back(new_ix);
                }

                self.next_warrior_id += 1;
            } else {
                let terminated_warrior = self.warriors_queues.remove(self.next_warrior_id);
                self.notify_observers(VmEvent {
                    event_type: EventType::TerminatedProgram,
                    moved_from: None,
                    offset: None,
                    warrior_id: terminated_warrior.warrior_id,
                    round: self.round,
                })
            }

            if self.next_warrior_id == self.warriors_queues.len() {
                self.next_warrior_id = 0;
                self.round += 1;
            }
        }

        if self.warriors_queues.len() == 1 {
            Some(&self.warriors_definitions[self.warriors_queues.iter().nth(0).unwrap().warrior_id])
        } else {
            None
        }
    }

    fn fold(
        &mut self,
        operand: Operand<CORE_SIZE>,
        instruction_pointer: Numeric<CORE_SIZE>,
        warrior_id: usize,
    ) -> Numeric<CORE_SIZE> {
        match operand.mode {
            OperandMode::Immediate => instruction_pointer,
            OperandMode::Direct => operand.pointer + instruction_pointer,
            OperandMode::Indirect => {
                let address = operand.pointer + instruction_pointer;

                address + self.core[address.value].b_operand.pointer
            }
            OperandMode::Increment => {
                let address = operand.pointer + instruction_pointer;
                let r = self.core[address.value].b_operand.pointer;
                self.core[address.value].b_operand.pointer += 1;

                self.notify_observers(VmEvent {
                    event_type: EventType::Change,
                    moved_from: None,
                    offset: Some(address.value),
                    warrior_id: warrior_id,
                    round: self.round,
                });

                address + r
            }
            OperandMode::Decrement => {
                let address = operand.pointer + instruction_pointer;
                self.core[address.value].b_operand.pointer -= 1;

                self.notify_observers(VmEvent {
                    event_type: EventType::Change,
                    moved_from: None,
                    offset: Some(address.value),
                    warrior_id: warrior_id,
                    round: self.round,
                });

                address + self.core[address.value].b_operand.pointer
            }
        }
    }

    fn execute(
        &mut self,
        operation: Instruction<CORE_SIZE>,
        instruction_pointer: Numeric<CORE_SIZE>,
        warrior_index: usize,
    ) -> Vec<Numeric<CORE_SIZE>> {
        let warrior_id = self.warriors_queues[warrior_index].warrior_id;
        let a_address = self.fold(operation.a_operand, instruction_pointer, warrior_id);
        let b_address = self.fold(operation.b_operand, instruction_pointer, warrior_id);

        let a_instruction = self.core[a_address.value];
        let b_instruction = self.core[b_address.value];
        match operation.op {
            OpCode::Dat => {
                self.notify_observers(VmEvent {
                    event_type: EventType::TerminatedThread,
                    moved_from: Some(instruction_pointer.value),
                    offset: None,
                    warrior_id: warrior_id,
                    round: self.round,
                });

                vec![]
            }
            OpCode::Mov => {
                match operation.modifier {
                    Modifier::A => self.core[b_address.value].a_operand = a_instruction.a_operand,
                    Modifier::B => self.core[b_address.value].b_operand = a_instruction.b_operand,
                    Modifier::AB => self.core[b_address.value].b_operand = a_instruction.a_operand,
                    Modifier::BA => self.core[b_address.value].a_operand = a_instruction.b_operand,
                    Modifier::F => {
                        self.core[b_address.value].a_operand = a_instruction.a_operand;
                        self.core[b_address.value].b_operand = a_instruction.b_operand;
                    }
                    Modifier::X => {
                        self.core[b_address.value].a_operand = a_instruction.b_operand;
                        self.core[b_address.value].b_operand = a_instruction.a_operand;
                    }
                    Modifier::I => self.core[b_address.value] = a_instruction,
                }

                self.notify_observers(VmEvent {
                    event_type: EventType::Change,
                    moved_from: None,
                    offset: Some(b_address.value),
                    warrior_id: warrior_id,
                    round: self.round,
                });

                vec![instruction_pointer + 1]
            }
            OpCode::Add => vec![self.handle_arithmetic(
                a_instruction,
                b_instruction,
                b_address,
                operation.modifier,
                instruction_pointer,
                sum,
                warrior_id,
            )],
            OpCode::Sub => vec![self.handle_arithmetic(
                a_instruction,
                b_instruction,
                b_address,
                operation.modifier,
                instruction_pointer,
                sub,
                warrior_id,
            )],
            OpCode::Mul => vec![self.handle_arithmetic(
                a_instruction,
                b_instruction,
                b_address,
                operation.modifier,
                instruction_pointer,
                mul,
                warrior_id,
            )],
            OpCode::Div => self.handle_div_arithmetic(
                a_instruction,
                b_instruction,
                b_address,
                operation.modifier,
                instruction_pointer,
                div,
                warrior_id,
            ),
            OpCode::Mod => self.handle_div_arithmetic(
                a_instruction,
                b_instruction,
                b_address,
                operation.modifier,
                instruction_pointer,
                rem,
                warrior_id,
            ),
            OpCode::Jmp => vec![a_address],
            OpCode::Jmz => match operation.modifier {
                Modifier::A | Modifier::BA if b_instruction.a_operand.pointer.value == 0 => {
                    vec![a_address]
                }
                Modifier::A | Modifier::BA => vec![instruction_pointer + 1],
                Modifier::B | Modifier::AB if b_instruction.b_operand.pointer.value == 0 => {
                    vec![a_address]
                }
                Modifier::B | Modifier::AB => vec![instruction_pointer + 1],
                Modifier::F | Modifier::X | Modifier::I
                    if b_instruction.a_operand.pointer.value == 0
                        && b_instruction.b_operand.pointer.value == 0 =>
                {
                    vec![a_address]
                }
                Modifier::F | Modifier::X | Modifier::I => vec![instruction_pointer + 1],
            },
            OpCode::Jmn => match operation.modifier {
                Modifier::A | Modifier::BA if b_instruction.a_operand.pointer.value == 0 => {
                    vec![instruction_pointer + 1]
                }
                Modifier::A | Modifier::BA => vec![a_address],
                Modifier::B | Modifier::AB if b_instruction.b_operand.pointer.value == 0 => {
                    vec![instruction_pointer + 1]
                }
                Modifier::B | Modifier::AB => vec![a_address],
                Modifier::F | Modifier::X | Modifier::I
                    if b_instruction.a_operand.pointer.value == 0
                        && b_instruction.b_operand.pointer.value == 0 =>
                {
                    vec![instruction_pointer + 1]
                }
                Modifier::F | Modifier::X | Modifier::I => vec![a_address],
            },
            OpCode::Djn => {
                let result = match operation.modifier {
                    Modifier::A | Modifier::BA => {
                        self.core[b_address.value].a_operand.pointer -= 1;
                        if self.core[b_address.value].a_operand.pointer.value != 0 {
                            vec![a_address]
                        } else {
                            vec![instruction_pointer + 1]
                        }
                    }
                    Modifier::B | Modifier::AB => {
                        self.core[b_address.value].b_operand.pointer -= 1;
                        if self.core[b_address.value].b_operand.pointer.value != 0 {
                            vec![a_address]
                        } else {
                            vec![instruction_pointer + 1]
                        }
                    }
                    Modifier::F | Modifier::X | Modifier::I => {
                        self.core[b_address.value].a_operand.pointer -= 1;
                        self.core[b_address.value].b_operand.pointer -= 1;

                        if self.core[b_address.value].a_operand.pointer.value != 0
                            || self.core[b_address.value].b_operand.pointer.value != 0
                        {
                            vec![a_address]
                        } else {
                            vec![instruction_pointer + 1]
                        }
                    }
                };

                self.notify_observers(VmEvent {
                    event_type: EventType::Change,
                    moved_from: None,
                    offset: Some(b_address.value),
                    warrior_id: warrior_id,
                    round: self.round,
                });

                result
            }
            OpCode::Cmp => match operation.modifier {
                Modifier::A
                    if self.core[b_address.value].a_operand.pointer
                        == self.core[a_address.value].a_operand.pointer =>
                {
                    vec![instruction_pointer + 2]
                }
                Modifier::A => vec![instruction_pointer + 1],
                Modifier::B
                    if self.core[b_address.value].b_operand.pointer
                        == self.core[a_address.value].b_operand.pointer =>
                {
                    vec![instruction_pointer + 2]
                }
                Modifier::B => vec![instruction_pointer + 1],
                Modifier::AB
                    if self.core[b_address.value].b_operand.pointer
                        == self.core[a_address.value].a_operand.pointer =>
                {
                    vec![instruction_pointer + 2]
                }
                Modifier::AB => vec![instruction_pointer + 1],
                Modifier::BA
                    if self.core[b_address.value].a_operand.pointer
                        == self.core[a_address.value].b_operand.pointer =>
                {
                    vec![instruction_pointer + 2]
                }
                Modifier::BA => vec![instruction_pointer + 1],
                Modifier::F
                    if self.core[b_address.value].a_operand.pointer
                        == self.core[a_address.value].a_operand.pointer
                        && self.core[b_address.value].b_operand.pointer
                            == self.core[a_address.value].b_operand.pointer =>
                {
                    vec![instruction_pointer + 2]
                }
                Modifier::F => vec![instruction_pointer + 1],
                Modifier::X
                    if self.core[b_address.value].a_operand.pointer
                        == self.core[a_address.value].b_operand.pointer
                        && self.core[b_address.value].b_operand.pointer
                            == self.core[a_address.value].a_operand.pointer =>
                {
                    vec![instruction_pointer + 2]
                }
                Modifier::X => vec![instruction_pointer + 1],
                Modifier::I if self.core[b_address.value] == self.core[a_address.value] => {
                    vec![instruction_pointer + 2]
                }
                Modifier::I => vec![instruction_pointer + 1],
            },
            OpCode::Slt => match operation.modifier {
                Modifier::A
                    if self.core[a_address.value].a_operand.pointer
                        < self.core[b_address.value].a_operand.pointer =>
                {
                    vec![instruction_pointer + 2]
                }
                Modifier::A => vec![instruction_pointer + 1],
                Modifier::B
                    if self.core[a_address.value].b_operand.pointer
                        < self.core[b_address.value].b_operand.pointer =>
                {
                    vec![instruction_pointer + 2]
                }
                Modifier::B => vec![instruction_pointer + 1],
                Modifier::AB
                    if self.core[a_address.value].a_operand.pointer
                        < self.core[b_address.value].b_operand.pointer =>
                {
                    vec![instruction_pointer + 2]
                }
                Modifier::AB => vec![instruction_pointer + 1],
                Modifier::BA
                    if self.core[a_address.value].b_operand.pointer
                        < self.core[b_address.value].a_operand.pointer =>
                {
                    vec![instruction_pointer + 2]
                }
                Modifier::BA => vec![instruction_pointer + 1],
                Modifier::F | Modifier::I
                    if self.core[a_address.value].a_operand.pointer
                        < self.core[b_address.value].a_operand.pointer
                        && self.core[a_address.value].b_operand.pointer
                            < self.core[b_address.value].b_operand.pointer =>
                {
                    vec![instruction_pointer + 2]
                }
                Modifier::F | Modifier::I => vec![instruction_pointer + 1],
                Modifier::X
                    if self.core[a_address.value].a_operand.pointer
                        < self.core[b_address.value].b_operand.pointer
                        && self.core[a_address.value].b_operand.pointer
                            < self.core[b_address.value].a_operand.pointer =>
                {
                    vec![instruction_pointer + 2]
                }
                Modifier::X => vec![instruction_pointer + 1],
            },
            OpCode::Spl => {
                if self.warriors_queues[warrior_index].instruction_queue.len() >= QUEUE_SIZE - 1 {
                    vec![instruction_pointer + 1]
                } else {
                    vec![instruction_pointer + 1, a_address]
                }
            }
        }
    }

    fn handle_arithmetic<F>(
        &mut self,
        a_instruction: Instruction<CORE_SIZE>,
        b_instruction: Instruction<CORE_SIZE>,
        b_address: Numeric<CORE_SIZE>,
        modifier: Modifier,
        instruction_pointer: Numeric<CORE_SIZE>,
        op: F,
        warrior_id: usize,
    ) -> Numeric<CORE_SIZE>
    where
        F: Fn(Numeric<CORE_SIZE>, Numeric<CORE_SIZE>) -> Numeric<CORE_SIZE>,
    {
        match modifier {
            Modifier::A => {
                self.core[b_address.value].a_operand.pointer = op(
                    b_instruction.a_operand.pointer,
                    a_instruction.a_operand.pointer,
                )
            }
            Modifier::B => {
                self.core[b_address.value].b_operand.pointer = op(
                    b_instruction.b_operand.pointer,
                    a_instruction.b_operand.pointer,
                )
            }
            Modifier::AB => {
                self.core[b_address.value].b_operand.pointer = op(
                    b_instruction.b_operand.pointer,
                    a_instruction.a_operand.pointer,
                )
            }
            Modifier::BA => {
                self.core[b_address.value].a_operand.pointer = op(
                    b_instruction.a_operand.pointer,
                    a_instruction.b_operand.pointer,
                )
            }
            Modifier::F | Modifier::I => {
                self.core[b_address.value].a_operand.pointer = op(
                    b_instruction.a_operand.pointer,
                    a_instruction.a_operand.pointer,
                );
                self.core[b_address.value].b_operand.pointer = op(
                    b_instruction.b_operand.pointer,
                    a_instruction.b_operand.pointer,
                );
            }
            Modifier::X => {
                self.core[b_address.value].b_operand.pointer = op(
                    b_instruction.b_operand.pointer,
                    a_instruction.a_operand.pointer,
                );
                self.core[b_address.value].a_operand.pointer = op(
                    b_instruction.a_operand.pointer,
                    a_instruction.b_operand.pointer,
                );
            }
        }

        self.notify_observers(VmEvent {
            event_type: EventType::Change,
            moved_from: None,
            offset: Some(b_address.value),
            warrior_id: warrior_id,
            round: self.round,
        });

        instruction_pointer + 1
    }

    fn handle_div_arithmetic<F>(
        &mut self,
        a_instruction: Instruction<CORE_SIZE>,
        b_instruction: Instruction<CORE_SIZE>,
        b_address: Numeric<CORE_SIZE>,
        modifier: Modifier,
        instruction_pointer: Numeric<CORE_SIZE>,
        op: F,
        warrior_id: usize,
    ) -> Vec<Numeric<CORE_SIZE>>
    where
        F: Fn(Numeric<CORE_SIZE>, Numeric<CORE_SIZE>) -> Numeric<CORE_SIZE>,
    {
        let result = match modifier {
            Modifier::A if a_instruction.a_operand.pointer.value != 0 => {
                self.core[b_address.value].a_operand.pointer = op(
                    b_instruction.a_operand.pointer,
                    a_instruction.a_operand.pointer,
                );

                vec![instruction_pointer + 1]
            }
            Modifier::B if a_instruction.b_operand.pointer.value != 0 => {
                self.core[b_address.value].b_operand.pointer = op(
                    b_instruction.b_operand.pointer,
                    a_instruction.b_operand.pointer,
                );

                vec![instruction_pointer + 1]
            }
            Modifier::AB if a_instruction.a_operand.pointer.value != 0 => {
                self.core[b_address.value].b_operand.pointer = op(
                    b_instruction.a_operand.pointer,
                    a_instruction.b_operand.pointer,
                );

                vec![instruction_pointer + 1]
            }
            Modifier::BA if a_instruction.b_operand.pointer.value != 0 => {
                self.core[b_address.value].a_operand.pointer = op(
                    b_instruction.b_operand.pointer,
                    a_instruction.a_operand.pointer,
                );

                vec![instruction_pointer + 1]
            }
            Modifier::F | Modifier::I => {
                if a_instruction.a_operand.pointer.value != 0 {
                    self.core[b_address.value].a_operand.pointer = op(
                        b_instruction.a_operand.pointer,
                        a_instruction.a_operand.pointer,
                    );
                }

                if a_instruction.b_operand.pointer.value != 0 {
                    self.core[b_address.value].b_operand.pointer = op(
                        b_instruction.b_operand.pointer,
                        a_instruction.b_operand.pointer,
                    );
                }

                if a_instruction.a_operand.pointer.value != 0
                    && a_instruction.b_operand.pointer.value != 0
                {
                    vec![instruction_pointer + 1]
                } else {
                    vec![]
                }
            }
            Modifier::X => {
                if a_instruction.a_operand.pointer.value != 0 {
                    self.core[b_address.value].b_operand.pointer = op(
                        b_instruction.b_operand.pointer,
                        a_instruction.a_operand.pointer,
                    );
                }

                if a_instruction.b_operand.pointer.value != 0 {
                    self.core[b_address.value].a_operand.pointer = op(
                        b_instruction.a_operand.pointer,
                        a_instruction.b_operand.pointer,
                    );
                }

                if a_instruction.a_operand.pointer.value != 0
                    && a_instruction.b_operand.pointer.value != 0
                {
                    vec![instruction_pointer + 1]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        };

        if result.len() > 0 {
            self.notify_observers(VmEvent {
                event_type: EventType::Change,
                moved_from: None,
                offset: Some(b_address.value),
                warrior_id: warrior_id,
                round: self.round,
            });
        }

        result
    }
}

fn sum<const CORE_SIZE: usize>(u: Numeric<CORE_SIZE>, i: Numeric<CORE_SIZE>) -> Numeric<CORE_SIZE> {
    u + i
}

fn sub<const CORE_SIZE: usize>(u: Numeric<CORE_SIZE>, i: Numeric<CORE_SIZE>) -> Numeric<CORE_SIZE> {
    u - i
}

fn mul<const CORE_SIZE: usize>(u: Numeric<CORE_SIZE>, i: Numeric<CORE_SIZE>) -> Numeric<CORE_SIZE> {
    u * i
}

fn div<const CORE_SIZE: usize>(u: Numeric<CORE_SIZE>, i: Numeric<CORE_SIZE>) -> Numeric<CORE_SIZE> {
    u / i
}

fn rem<const CORE_SIZE: usize>(u: Numeric<CORE_SIZE>, i: Numeric<CORE_SIZE>) -> Numeric<CORE_SIZE> {
    u % i
}
