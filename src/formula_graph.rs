use crate::elf::ElfMetadata;
use crate::iterator::ForEachUntilSome;
use byteorder::{ByteOrder, LittleEndian};
use core::fmt;
use petgraph::graph::NodeIndex;
use petgraph::Graph;
use riscv_decode::types::*;
use riscv_decode::Instruction;

pub type Formula = Graph<Node, ArgumentSide>;

static REG_SP: usize = 2;
static REG_A0: usize = 10;
static REG_A1: usize = 11;
static REG_A2: usize = 12;
static REG_A7: usize = 17;

#[allow(dead_code)]
pub enum SyscallId {
    Exit = 93,
    Read = 63,
    Write = 64,
    Openat = 56,
    Brk = 214,
}

fn instruction_to_str(i: Instruction) -> &'static str {
    match i {
        Instruction::Lui(_) => "lui",
        Instruction::Jal(_) => "jal",
        Instruction::Jalr(_) => "jalr",
        Instruction::Beq(_) => "beq",
        Instruction::Ld(_) => "ld",
        Instruction::Sd(_) => "sd",
        Instruction::Addi(_) => "addi",
        Instruction::Add(_) => "add",
        Instruction::Sub(_) => "sub",
        Instruction::Sltu(_) => "sltu",
        Instruction::Mul(_) => "mul",
        Instruction::Divu(_) => "divu",
        Instruction::Remu(_) => "remu",
        Instruction::Ecall => "ecall",
        _ => "unknown",
    }
}

#[derive(Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub enum ArgumentSide {
    Lhs,
    Rhs,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Instr {
    // These instructions are part of the formula graph
    // IE = input edge
    // OE = output edge
    // Lui(utype) -> 0 IE / 1 OE
    // Addi(itype) -> 1 IE / 1 OE
    // Add(rtype) -> 2 IE / 1 OE
    // Sub(rtype) -> 2 IE / 1 OE
    // Mul(rtype) -> 2 IE / 1 OE
    // Divu(rtype) -> 2 IE / 1 OE
    // Remu(rtype) -> 2 IE / 1 OE
    // Sltu(rtype) -> 2 IE / 1 OE
    instruction: Instruction,
}

impl Instr {
    fn new(instruction: Instruction) -> Self {
        Self { instruction }
    }
}

impl fmt::Debug for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.instruction)
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Const {
    // can have multiple output edges, but no input edge
    value: u64,
}

impl Const {
    fn new(value: u64) -> Self {
        Self { value }
    }
}

impl fmt::Debug for Const {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Input {
    // can have multiple output edges, but no input edge
    name: String,
}

impl Input {
    fn new(name: String) -> Self {
        Self { name }
    }
}

impl fmt::Debug for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub enum BooleanFunction {
    // Equals,
    GreaterThan,
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Constrain {
    // has 1 input edge only and 0 output edges
    name: String,
    op: BooleanFunction,
}

impl Constrain {
    fn new(name: String, op: BooleanFunction) -> Self {
        Self { name, op }
    }
}

impl fmt::Debug for Constrain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Node {
    Instruction(Instr),
    Constant(Const),
    Input(Input),
    Constrain(Constrain),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Value {
    Concrete(u64),
    Symbolic(NodeIndex),
    Uninitialized,
}

struct DataFlowGraphBuilder<'a> {
    graph: Formula,
    path: &'a [Instruction],
    program_break: u64,
    regs: [Value; 32],
    memory: Vec<Value>,
}

impl<'a> DataFlowGraphBuilder<'a> {
    // creates a machine state with a specifc memory size
    fn new(
        memory_size: usize,
        path: &'a [Instruction],
        data_segment: &[u8],
        elf_metadata: ElfMetadata,
    ) -> Self {
        let mut regs = [Value::Concrete(0); 32];
        let mut memory = vec![Value::Uninitialized; memory_size / 8];

        regs[REG_SP] = Value::Concrete(memory_size as u64 - 8);

        println!(
            "data_segment.len(): {}   entry_address:  {}",
            data_segment.len(),
            elf_metadata.entry_address
        );

        let start = (elf_metadata.entry_address / 8) as usize;
        let end = start + data_segment.len() / 8;

        data_segment
            .chunks(8)
            .map(LittleEndian::read_u64)
            .zip(start..end)
            .for_each(|(x, i)| memory[i] = Value::Concrete(x));

        Self {
            graph: Formula::new(),
            program_break: elf_metadata.entry_address + (data_segment.len() as u64),
            path,
            regs,
            memory,
        }
    }

    fn create_const_node(&mut self, value: u64) -> NodeIndex {
        let constant = Node::Constant(Const::new(value));

        self.graph.add_node(constant)
    }

    fn symbolic_op(&mut self, lhs: NodeIndex, rhs: NodeIndex, result: NodeIndex) -> Value {
        self.graph.add_edge(lhs, result, ArgumentSide::Lhs);
        self.graph.add_edge(rhs, result, ArgumentSide::Rhs);

        Value::Symbolic(result)
    }

    fn execute_lui(&mut self, utype: UType) -> Option<NodeIndex> {
        if utype.rd() == 0 {
            return None;
        }

        let immediate = utype.imm() as u64; //sign_extend_utype(utype.imm());

        let result = Value::Concrete(immediate);

        println!(
            "{}  imm: {:?} -> rd: {:?}",
            instruction_to_str(Instruction::Lui(utype)),
            immediate as i64,
            result,
        );

        self.regs[utype.rd() as usize] = result;

        None
    }

    fn execute_itype<Op>(
        &mut self,
        instruction: Instruction,
        itype: IType,
        op: Op,
    ) -> Option<NodeIndex>
    where
        Op: FnOnce(u64, u64) -> u64,
    {
        if itype.rd() == 0 {
            return None;
        }

        let rs1_value = self.regs[itype.rs1() as usize];
        let immediate = sign_extend_itype_stype(itype.imm());

        let result = self.execute_binary_op(instruction, rs1_value, Value::Concrete(immediate), op);

        println!(
            "{}  rs1: {:?} imm: {:?} -> rd: {:?}",
            instruction_to_str(instruction),
            rs1_value,
            immediate as i64,
            result,
        );

        self.regs[itype.rd() as usize] = result;

        None
    }

    fn execute_rtype<Op>(
        &mut self,
        instruction: Instruction,
        rtype: RType,
        op: Op,
    ) -> Option<NodeIndex>
    where
        Op: FnOnce(u64, u64) -> u64,
    {
        if rtype.rd() == 0 {
            return None;
        }

        let rs1_value = self.regs[rtype.rs1() as usize];
        let rs2_value = self.regs[rtype.rs2() as usize];

        let result = self.execute_binary_op(instruction, rs1_value, rs2_value, op);

        println!(
            "{}  rs1: {:?} rs2: {:?} -> rd: {:?}",
            instruction_to_str(instruction),
            rs1_value,
            rs2_value,
            result,
        );

        self.regs[rtype.rd() as usize] = result;

        None
    }

    fn create_result_node(&mut self, instruction: Instruction) -> NodeIndex {
        let node = Node::Instruction(Instr::new(instruction));

        self.graph.add_node(node)
    }

    fn execute_binary_op<Op>(
        &mut self,
        instruction: Instruction,
        lhs: Value,
        rhs: Value,
        op: Op,
    ) -> Value
    where
        Op: FnOnce(u64, u64) -> u64,
    {
        match (lhs, rhs) {
            (Value::Concrete(v1), Value::Concrete(v2)) => Value::Concrete(op(v1, v2)),
            (Value::Symbolic(v1), Value::Concrete(v2)) => {
                let node = self.create_const_node(v2);
                let res = self.create_result_node(instruction);
                self.symbolic_op(v1, node, res)
            }
            (Value::Concrete(v1), Value::Symbolic(v2)) => {
                let node = self.create_const_node(v1);
                let res = self.create_result_node(instruction);
                self.symbolic_op(node, v2, res)
            }
            (Value::Symbolic(v1), Value::Symbolic(v2)) => {
                let res = self.create_result_node(instruction);
                self.symbolic_op(v1, v2, res)
            }
            // TODO: generate exit node here
            _ => panic!("access to unitialized memory"),
        }
    }

    pub fn generate_graph(&mut self) -> Option<(Formula, NodeIndex)> {
        if let Some(root_idx) = self
            .path
            .iter()
            .for_each_until_some(|instr| self.execute(*instr))
        {
            Some((self.graph.clone(), root_idx))
        } else {
            None
        }
    }

    fn execute_brk(&mut self) -> Option<NodeIndex> {
        if let Value::Concrete(new_program_break) = self.regs[REG_A0] {
            // TODO: handle cases where program break can not be modified
            if new_program_break < self.program_break {
                self.regs[REG_A0] = Value::Concrete(self.program_break);
            } else {
                self.program_break = new_program_break;
            }
            println!("new program break: {}", new_program_break);
        } else {
            unimplemented!("can not handle symbolic or uninitialized program break")
        }
        None
    }

    fn execute_read(&mut self) -> Option<NodeIndex> {
        // TODO: ignore FD??
        if let Value::Concrete(buffer) = self.regs[REG_A1] {
            if let Value::Concrete(size) = self.regs[REG_A2] {
                // assert!(
                //     size % 8 == 0,
                //     "can only handle read syscalls with word width"
                // );
                // TODO: round up to word width.. not the best idea, right???
                let to_add = 8 - (size % 8);
                let words_read = (size + to_add) / 8;

                for i in 0..words_read {
                    let name = format!("read({}, {}, {})", 0, buffer, size);
                    let node = Node::Input(Input::new(name));
                    let node_idx = self.graph.add_node(node);
                    self.memory[((buffer / 8) + i) as usize] = Value::Symbolic(node_idx);
                }
            } else {
                unimplemented!("can not handle symbolic or uinitialized size in read syscall")
            }
        } else {
            unimplemented!(
                "can not handle symbolic or uninitialized buffer address in read syscall"
            )
        }
        None
    }

    fn execute_exit(&mut self) -> Option<NodeIndex> {
        if let Value::Symbolic(exit_code) = self.regs[REG_A0] {
            let const_node = Node::Constant(Const::new(0));
            let const_node_idx = self.graph.add_node(const_node);

            let root = Node::Constrain(Constrain::new(
                String::from("exit_code"),
                BooleanFunction::GreaterThan,
            ));
            let root_idx = self.graph.add_node(root);

            self.graph.add_edge(exit_code, root_idx, ArgumentSide::Lhs);
            self.graph
                .add_edge(const_node_idx, root_idx, ArgumentSide::Rhs);

            Some(root_idx)
        } else {
            unimplemented!("exit only implemented for symbolic exit codes")
        }
    }

    fn execute_ecall(&mut self) -> Option<NodeIndex> {
        match self.regs[REG_A7] {
            Value::Concrete(syscall_id) if syscall_id == (SyscallId::Brk as u64) => {
                self.execute_brk()
            }
            Value::Concrete(syscall_id) if syscall_id == (SyscallId::Read as u64) => {
                self.execute_read()
            }
            Value::Concrete(syscall_id) if syscall_id == (SyscallId::Exit as u64) => {
                self.execute_exit()
            }
            Value::Concrete(x) => unimplemented!("this syscall ({}) is not implemented yet", x),
            Value::Uninitialized => unimplemented!("ecall with uninitialized syscall id"),
            Value::Symbolic(_) => unimplemented!("ecall with symbolic syscall id not implemented"),
        }
    }

    fn execute_load(&mut self, instruction: Instruction, itype: IType) -> Option<NodeIndex> {
        if itype.rd() != 0 {
            if let Value::Concrete(base_address) = self.regs[itype.rs1() as usize] {
                let immediate = sign_extend_itype_stype(itype.imm());

                let address = base_address.wrapping_add(immediate);

                let value = self.memory[(address / 8) as usize];

                println!(
                    "{} rs1: {:?} imm: {} -> rd: {:?}",
                    instruction_to_str(instruction),
                    self.regs[itype.rs1() as usize],
                    immediate as i64,
                    value,
                );

                self.regs[itype.rd() as usize] = value;
            } else {
                unimplemented!("can not handle symbolic addresses in LD")
            }
        }

        None
    }

    fn execute_store(&mut self, instruction: Instruction, stype: SType) -> Option<NodeIndex> {
        if let Value::Concrete(base_address) = self.regs[stype.rs1() as usize] {
            let immediate = sign_extend_itype_stype(stype.imm());

            let address = base_address.wrapping_add(immediate);

            let value = self.regs[stype.rs2() as usize];

            println!(
                "{}  immediate: {:?} rs2: {:?} rs1: {:?} -> ",
                instruction_to_str(instruction),
                immediate as i64,
                self.regs[stype.rs1() as usize],
                value,
            );

            self.memory[(address / 8) as usize] = value;
        } else {
            unimplemented!("can not handle symbolic addresses in SD")
        }

        None
    }

    fn execute(&mut self, instruction: Instruction) -> Option<NodeIndex> {
        match instruction {
            Instruction::Ecall => self.execute_ecall(),
            Instruction::Lui(utype) => self.execute_lui(utype),
            Instruction::Addi(itype) => self.execute_itype(instruction, itype, u64::wrapping_add),
            Instruction::Add(rtype) => self.execute_rtype(instruction, rtype, u64::wrapping_add),
            Instruction::Sub(rtype) => self.execute_rtype(instruction, rtype, u64::wrapping_sub),
            Instruction::Mul(rtype) => self.execute_rtype(instruction, rtype, u64::wrapping_mul),
            // TODO: Implement exit for DIVU
            Instruction::Divu(rtype) => self.execute_rtype(instruction, rtype, u64::wrapping_div),
            Instruction::Remu(rtype) => self.execute_rtype(instruction, rtype, u64::wrapping_rem),
            Instruction::Sltu(rtype) => {
                self.execute_rtype(instruction, rtype, |l, r| if l < r { 1 } else { 0 })
            }
            Instruction::Ld(itype) => self.execute_load(instruction, itype),
            Instruction::Sd(stype) => self.execute_store(instruction, stype),
            Instruction::Jal(jtype) => {
                if jtype.rd() != 0 {
                    self.regs[jtype.rd() as usize] = Value::Concrete(0);
                }
                None
            }
            Instruction::Jalr(itype) => {
                if itype.rd() != 0 {
                    self.regs[itype.rd() as usize] = Value::Concrete(0);
                }
                None
            }
            Instruction::Beq(_btype) => None,
            _ => unimplemented!("can not handle this instruction"),
        }
    }
}

pub fn sign_extend(n: u64, b: u32) -> u64 {
    // assert: 0 <= n <= 2^b
    // assert: 0 < b < CPUBITWIDTH
    if n < 2_u64.pow(b - 1) {
        n
    } else {
        n.wrapping_sub(2_u64.pow(b))
    }
}

#[allow(dead_code)]
fn sign_extend_utype(imm: u32) -> u64 {
    sign_extend(imm as u64, 20)
}

fn sign_extend_itype_stype(imm: u32) -> u64 {
    sign_extend(imm as u64, 12)
}

#[allow(dead_code)]
fn build_dataflow_graph(
    path: &[Instruction],
    data_segment: &[u8],
    elf_metadata: ElfMetadata,
) -> Option<(Formula, NodeIndex)> {
    DataFlowGraphBuilder::new(1000000, path, data_segment, elf_metadata).generate_graph()
}

// TODO: need to load data segment  => then write test
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg;
    use crate::cfg::ControlFlowGraph;
    use crate::dead_code_elimination::eliminate_dead_code;
    use petgraph::dot::Dot;
    use petgraph::visit::EdgeRef;
    use serial_test::serial;
    use std::env::current_dir;
    use std::fs::File;
    use std::io::prelude::*;
    use std::path::Path;
    use std::process::Command;

    // Returns a path of RISC-U instructions and branch decisions (if true or false branch has been taken)
    // for a path with 1 BEQ instruction, the vector of branch decisions has the length of 1
    pub fn extract_candidate_path(graph: &ControlFlowGraph) -> (Vec<Instruction>, Vec<bool>) {
        fn next(graph: &ControlFlowGraph, idx: NodeIndex) -> Option<(NodeIndex, Option<bool>)> {
            let edges = graph.edges(idx);

            if let Some(edge) = edges.last() {
                let target = edge.target();

                match graph[idx] {
                    Instruction::Beq(_) => {
                        let next_idx = edge.target().index();

                        if next_idx == idx.index() + 1 {
                            Some((target, Some(false)))
                        } else {
                            Some((target, Some(true)))
                        }
                    }
                    _ => Some((target, None)),
                }
            } else {
                None
            }
        }
        let mut path = vec![];
        let mut branch_decisions = vec![];
        let mut idx = graph.node_indices().next().unwrap();
        path.push(idx);
        while let Some(n) = next(graph, idx) {
            path.push(n.0);
            idx = n.0;

            if let Some(branch_decision) = n.1 {
                branch_decisions.push(branch_decision);
            }
        }
        let instruction_path = path.iter().map(|idx| graph[*idx]).collect();

        (instruction_path, branch_decisions)
    }

    // TODO: write a unit test without dependency on selfie and external files
    #[test]
    #[serial]
    fn can_build_formula() {
        let cd = String::from(current_dir().unwrap().to_str().unwrap());

        // generate RISC-U binary with Selfie
        let _ = Command::new("docker")
            .arg("run")
            .arg("-v")
            .arg(cd + ":/opt/monster")
            .arg("cksystemsteaching/selfie")
            .arg("/opt/selfie/selfie")
            .arg("-c")
            .arg("/opt/monster/symbolic/symbolic-exit.c")
            .arg("-o")
            .arg("/opt/monster/symbolic/symbolic-exit.riscu.o")
            .output();

        let test_file = Path::new("symbolic/symbolic-exit.riscu.o");

        let (graph, data_segment, elf_metadata) = cfg::build_from_file(test_file).unwrap();

        println!("{:?}", data_segment);

        let (path, _branch_decisions) = extract_candidate_path(&graph);

        println!("{:?}", path);

        let (formula, _root) =
            build_dataflow_graph(&path, data_segment.as_slice(), elf_metadata).unwrap();

        let graph_wo_dc = eliminate_dead_code(&formula, _root);

        let dot_graph = Dot::with_config(&graph_wo_dc, &[]);

        let mut f = File::create("tmp-graph.dot").unwrap();
        f.write_fmt(format_args!("{:?}", dot_graph)).unwrap();

        let _ = Command::new("dot")
            .arg("-Tpng")
            .arg("tmp-graph.dot")
            .arg("-o")
            .arg("main_wo_dc.png")
            .output();

        let dot_graph = Dot::with_config(&formula, &[]);

        let mut f = File::create("tmp-graph.dot").unwrap();
        f.write_fmt(format_args!("{:?}", dot_graph)).unwrap();

        let _ = Command::new("dot")
            .arg("-Tpng")
            .arg("tmp-graph.dot")
            .arg("-o")
            .arg("main.png")
            .output();

        // TODO: test more than just this result
        // assert!(result.is_ok());

        let _ = std::fs::remove_file(test_file);
    }
}
