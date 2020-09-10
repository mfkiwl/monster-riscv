use crate::formula_graph::{
    ArgumentSide, BooleanFunction, Formula,
    Node::{Constant, Constrain, Input, Instruction},
};
use boolector::{
    option::{BtorOption, ModelGen, OutputFileFormat},
    Btor, BV,
};
use petgraph::{graph::NodeIndex, Direction};
use riscv_decode::Instruction as Inst;
use std::rc::Rc;

fn get_operands(
    graph: &Formula,
    node: &NodeIndex,
    solver: &Rc<Btor>,
) -> (BV<Rc<Btor>>, BV<Rc<Btor>>) {
    let mut operands = graph
        .neighbors_directed(*node, Direction::Incoming)
        .detach();

    match operands.next(graph) {
        Some(p) if graph[p.0] == ArgumentSide::Lhs => (
            traverse(graph, &p.1, solver),
            traverse(graph, &operands.next(graph).unwrap().1, solver),
        ),
        Some(p) if graph[p.0] == ArgumentSide::Rhs => (
            traverse(graph, &p.1, solver),
            traverse(graph, &operands.next(graph).unwrap().1, solver),
        ),
        _ => unreachable!(),
    }
}

fn traverse<'a>(graph: &Formula, node: &NodeIndex, solver: &'a Rc<Btor>) -> BV<Rc<Btor>> {
    let bv = match &graph[*node] {
        Instruction(i) => {
            println!("instruction: {:?}", i);

            let (lhs, rhs) = get_operands(graph, node, solver);

            println!("instruction operand lhs: {:?}", lhs);
            println!("instruction operand rhs: {:?}", rhs);

            match i.instruction {
                Inst::Sub(i) => {
                    println!("sub {:?}", i);
                    lhs.sub(&rhs)
                }
                Inst::Addi(i) => {
                    println!("addi {:?}", i);
                    lhs.add(&rhs)
                }
                Inst::Add(i) => {
                    println!("add {:?}", i);
                    lhs.add(&rhs)
                }
                Inst::Mul(i) => {
                    println!("mul {:?}", i);
                    lhs.mul(&rhs)
                }
                i => unimplemented!("instruction: {:?}", i),
            }
        }
        Constrain(i) => {
            println!("constraint: {:?}", i);

            let (lhs, rhs) = get_operands(graph, node, solver);

            println!("constraint lhs: {:?}", lhs);
            println!("constraint rhs: {:?}", rhs);

            match i.op {
                BooleanFunction::GreaterThan => {
                    println!("{:?} > {:?}", lhs, rhs);
                    lhs.ugt(&rhs)
                }
            }
        }
        Input(i) => {
            // TODO: use size of read size / 8
            println!("input: {:?}", i);
            BV::new(solver.clone(), 8, None)
        }
        Constant(i) => {
            println!("constant: {}", i.value);
            BV::from_u64(solver.clone(), i.value, 8)
        }
    };

    println!("partial formula:\n{:?}", bv);

    bv
}

pub fn smt(graph: &Formula) {
    graph.externals(Direction::Outgoing).for_each(|n| {
        let solver = Rc::new(Btor::new());
        solver.set_opt(BtorOption::ModelGen(ModelGen::All));
        solver.set_opt(BtorOption::Incremental(true));
        solver.set_opt(BtorOption::OutputFileFormat(OutputFileFormat::SMTLIBv2));

        if let Constrain(_) = &graph[n] {
            traverse(graph, &n, &solver).assert();

            println!("solver:");
            print!("{}", solver.print_constraints());
            println!("result: {:?}", solver.sat());
            println!();
        }
    });
}
