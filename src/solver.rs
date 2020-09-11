#![allow(dead_code)]
#![allow(unused_variables)]

use crate::bitvec::*;
use crate::formula_graph::*;
use crate::ternary::*;
use riscv_decode::Instruction;

// check if invertability condition is met
fn is_invertable(
    instruction: Instruction,
    x: TernaryBitVector,
    s: BitVector,
    t: BitVector,
) -> bool {
    match instruction {
        Instruction::Add(_) => x.mcb(s - t),
        _ => unimplemented!(),
    }
}

// can only handle one Equals constrain with constant
fn solve(formula: &Formula) -> Option<BitVector> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_formula_with_input() -> (Formula, NodeIndex) {
        let mut formula = Formula::new();

        let input = Node::Input(Input::new("x0".to_string()));
        let input_idx = formula.add_node(input);

        (formula, input_idx)
    }

    fn add_equals_constrain(formula: &mut Formula, to: NodeIndex, on: ArgumentSide, constant: u64) {
        let constrain =
            Node::Constrain(Constrain::new("exit".to_string(), BooleanFunction::Equals));
        let constrain_idx = formula.add_node(constrain);

        let constrain_c = Node::Constant(Const::new(10));
        let constrain_c_idx = formula.add_node(constrain_c);

        formula.add_edge(to, constrain_idx, on);
        formula.add_edge(constrain_c_idx, constrain_idx, on.other());
    }

    #[test]
    fn solve_trivial_equals_constrain() {
        let (mut formula, input_idx) = create_formula_with_input();

        add_equals_constrain(&mut formula, input_idx, ArgumentSide::Lhs, 10);
    }

    //#[test]
    //fn solve_bvadd() {

    //let constant = Node::Constant(Const::new(5));
    //let constant_idx = formula.add_node(constant);

    //let instr = Node::Instruction(Instr::new(Instruction::Add(RType(0)))); // registers do not mather
    //let instr_idx = formula.add_node(instr);

    //formula.add_edge(input_idx, instr_idx, ArgumentSide::Lhs);
    //formula.add_edge(constant_idx, instr_idx, ArgumentSide::Rhs);

    // output should equal 10 => there is an assignment: x0 = 5

    //}
}
