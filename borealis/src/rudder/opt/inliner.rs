use common::HashMap;

use crate::rudder::{Block, Function, Statement, StatementBuilder, StatementKind, Type};

const INLINE_SIZE_THRESHOLD: usize = 5;

pub fn run(f: Function) -> bool {
    do_block_inlining(&f)
}

fn do_block_inlining(f: &Function) -> bool {
    let mut changed = false;

    for block in f.entry_block().iter() {
        changed |= inline_target_block(block);
    }

    changed
}

fn clone_statement(
    builder: &mut StatementBuilder,
    template: &Statement,
    mapping: &HashMap<Statement, Statement>,
) -> Statement {
    match template.kind() {
        StatementKind::BinaryOperation { kind, lhs, rhs } => {
            builder.build(StatementKind::BinaryOperation {
                kind,
                lhs: mapping.get(&lhs).unwrap().clone(),
                rhs: mapping.get(&rhs).unwrap().clone(),
            })
        }
        StatementKind::Constant { typ, value } => {
            builder.build(StatementKind::Constant { typ, value })
        }
        StatementKind::ReadVariable { symbol } => {
            builder.build(StatementKind::ReadVariable { symbol })
        }
        StatementKind::WriteVariable { symbol, value } => {
            builder.build(StatementKind::WriteVariable {
                symbol,
                value: mapping.get(&value).unwrap().clone(),
            })
        }
        StatementKind::ReadRegister { typ, offset } => builder.build(StatementKind::ReadRegister {
            typ,
            offset: mapping.get(&offset).unwrap().clone(),
        }),
        StatementKind::WriteRegister { offset, value } => {
            builder.build(StatementKind::WriteRegister {
                offset: mapping.get(&offset).unwrap().clone(),
                value: mapping.get(&value).unwrap().clone(),
            })
        }
        StatementKind::ReadMemory { offset, size } => builder.build(StatementKind::ReadMemory {
            offset: mapping.get(&offset).unwrap().clone(),
            size: mapping.get(&size).unwrap().clone(),
        }),
        StatementKind::WriteMemory { offset, value } => builder.build(StatementKind::WriteMemory {
            offset: mapping.get(&offset).unwrap().clone(),
            value: mapping.get(&value).unwrap().clone(),
        }),
        StatementKind::ReadPc => builder.build(StatementKind::ReadPc),
        StatementKind::WritePc { value } => builder.build(StatementKind::WritePc {
            value: mapping.get(&value).unwrap().clone(),
        }),
        StatementKind::UnaryOperation { kind, value } => {
            builder.build(StatementKind::UnaryOperation {
                kind,
                value: mapping.get(&value).unwrap().clone(),
            })
        }
        StatementKind::ShiftOperation {
            kind,
            value,
            amount,
        } => builder.build(StatementKind::ShiftOperation {
            kind,
            value: mapping.get(&value).unwrap().clone(),
            amount: mapping.get(&amount).unwrap().clone(),
        }),
        StatementKind::Call { target, args, tail } => {
            let args = args
                .iter()
                .map(|stmt| mapping.get(stmt).unwrap().clone())
                .collect();

            builder.build(StatementKind::Call { target, args, tail })
        }
        StatementKind::Cast { kind, typ, value } => builder.build(StatementKind::Cast {
            kind,
            typ: typ.clone(),
            value: mapping.get(&value).unwrap().clone(),
        }),
        StatementKind::BitsCast {
            kind,
            typ,
            value,
            length,
        } => builder.build(StatementKind::BitsCast {
            kind,
            typ: typ.clone(),
            value: mapping.get(&value).unwrap().clone(),
            length: mapping.get(&length).unwrap().clone(),
        }),
        StatementKind::Jump { target } => builder.build(StatementKind::Jump { target }),
        StatementKind::Branch {
            condition,
            true_target,
            false_target,
        } => builder.build(StatementKind::Branch {
            condition: mapping.get(&condition).unwrap().clone(),
            true_target,
            false_target,
        }),
        StatementKind::PhiNode { members } => todo!(),
        StatementKind::Return { value: Some(value) } => builder.build(StatementKind::Return {
            value: Some(mapping.get(&value).unwrap().clone()),
        }),
        StatementKind::Return { value: None } => {
            builder.build(StatementKind::Return { value: None })
        }
        StatementKind::Select {
            condition,
            true_value,
            false_value,
        } => builder.build(StatementKind::Select {
            condition: mapping.get(&condition).unwrap().clone(),
            true_value: mapping.get(&true_value).unwrap().clone(),
            false_value: mapping.get(&false_value).unwrap().clone(),
        }),
        StatementKind::BitExtract {
            value,
            start,
            length,
        } => builder.build(StatementKind::BitExtract {
            value: mapping.get(&value).unwrap().clone(),
            start: mapping.get(&start).unwrap().clone(),
            length: mapping.get(&length).unwrap().clone(),
        }),
        StatementKind::BitInsert {
            original_value,
            insert_value,
            start,
            length,
        } => builder.build(StatementKind::BitInsert {
            original_value: mapping.get(&original_value).unwrap().clone(),
            insert_value: mapping.get(&insert_value).unwrap().clone(),
            start: mapping.get(&start).unwrap().clone(),
            length: mapping.get(&length).unwrap().clone(),
        }),
        StatementKind::ReadElement { vector, index } => builder.build(StatementKind::ReadElement {
            vector: mapping.get(&vector).unwrap().clone(),
            index: mapping.get(&index).unwrap().clone(),
        }),
        StatementKind::MutateElement {
            vector,
            value,
            index,
        } => builder.build(StatementKind::MutateElement {
            vector: mapping.get(&vector).unwrap().clone(),
            value: mapping.get(&value).unwrap().clone(),
            index: mapping.get(&index).unwrap().clone(),
        }),
        StatementKind::Panic(stmts) => {
            let stmts = stmts
                .iter()
                .map(|stmt| mapping.get(stmt).unwrap().clone())
                .collect();

            builder.build(StatementKind::Panic(stmts))
        }
        StatementKind::Assert { condition } => builder.build(StatementKind::Assert {
            condition: mapping.get(&condition).unwrap().clone(),
        }),
        StatementKind::CreateProduct { typ, fields } => {
            let fields = fields
                .iter()
                .map(|stmt| mapping.get(stmt).unwrap().clone())
                .collect();

            builder.build(StatementKind::CreateProduct { typ, fields })
        }
        StatementKind::CreateSum {
            typ,
            variant,
            value,
        } => builder.build(StatementKind::CreateSum {
            typ,
            variant,
            value: mapping.get(&value).unwrap().clone(),
        }),
        StatementKind::CreateBits { value, length } => builder.build(StatementKind::CreateBits {
            value: mapping.get(&value).unwrap().clone(),
            length: mapping.get(&length).unwrap().clone(),
        }),
        StatementKind::SizeOf { value } => builder.build(StatementKind::SizeOf {
            value: mapping.get(&value).unwrap().clone(),
        }),
        StatementKind::MatchesSum {
            value,
            variant_index,
        } => builder.build(StatementKind::MatchesSum {
            value: mapping.get(&value).unwrap().clone(),
            variant_index,
        }),
        StatementKind::UnwrapSum {
            value,
            variant_index,
        } => builder.build(StatementKind::UnwrapSum {
            value: mapping.get(&value).unwrap().clone(),
            variant_index,
        }),
        StatementKind::ExtractField { value, field_index } => {
            builder.build(StatementKind::ExtractField {
                value: mapping.get(&value).unwrap().clone(),
                field_index,
            })
        }
        StatementKind::UpdateField {
            original_value,
            field_index,
            field_value,
        } => builder.build(StatementKind::UpdateField {
            original_value: mapping.get(&original_value).unwrap().clone(),
            field_index,
            field_value: mapping.get(&field_value).unwrap().clone(),
        }),
    }
}

fn inline_target_block(source_block: Block) -> bool {
    // if a block ends in a jump statement, and the target block is "small", inline it.
    let terminator = source_block.terminator_statement().unwrap();

    let StatementKind::Jump {
        target: target_block,
    } = terminator.kind()
    else {
        return false;
    };

    if target_block.size() > INLINE_SIZE_THRESHOLD {
        return false;
    }

    // kill the jump statement, copy target block statements in.
    source_block.kill_statement(&terminator);

    let mut builder = StatementBuilder::new(source_block.weak());

    let mut mapping = HashMap::default();
    for stmt in target_block.statements() {
        let cloned_stmt = clone_statement(&mut builder, &stmt, &mapping);
        mapping.insert(stmt, cloned_stmt.clone());
    }

    source_block.extend_statements(builder.finish().into_iter());

    true
}
