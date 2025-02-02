use ir_to_bytecode::{compiler::compile_module, parser::parse_module};
use stackless_bytecode_generator::{
    stackless_bytecode::StacklessBytecode::{self, *},
    stackless_bytecode_generator::StacklessModuleGenerator,
};
use types::account_address::AccountAddress;
use vm::file_format::{
    AddressPoolIndex, ByteArrayPoolIndex, CompiledModule, FieldDefinitionIndex,
    FunctionHandleIndex, SignatureToken, StructDefinitionIndex, StructHandleIndex,
};

#[test]
fn transform_code_with_refs() {
    let code = String::from(
        "
        module Foobar {
            resource T { value: u64 }

            public all_about_refs(a: &R#Self.T, b: &mut u64, c: &mut R#Self.T): u64 {
                let value_ref: &u64;
                let frozen_ref: &R#Self.T;
                *move(b) = 0;
                value_ref = &move(a).value;
                frozen_ref = freeze(move(c));
                release(move(frozen_ref));
                return *move(value_ref);
            }
        }
        ",
    );

    let (actual_code, actual_types) = generate_code_from_string(code);
    let expected_code = vec![
        LdConst(5, 0),
        MoveLoc(6, 1),
        WriteRef(6, 5),
        MoveLoc(7, 0),
        BorrowField(8, 7, FieldDefinitionIndex::new(0)),
        StLoc(3, 8),
        MoveLoc(9, 2),
        FreezeRef(10, 9),
        StLoc(4, 10),
        MoveLoc(11, 4),
        ReleaseRef(11),
        MoveLoc(12, 3),
        ReadRef(13, 12),
        Ret(vec![13]),
    ];
    let expected_types = vec![
        SignatureToken::Reference(Box::new(SignatureToken::Struct(
            StructHandleIndex::new(0),
            vec![],
        ))),
        SignatureToken::MutableReference(Box::new(SignatureToken::U64)),
        SignatureToken::MutableReference(Box::new(SignatureToken::Struct(
            StructHandleIndex::new(0),
            vec![],
        ))),
        SignatureToken::Reference(Box::new(SignatureToken::U64)),
        SignatureToken::Reference(Box::new(SignatureToken::Struct(
            StructHandleIndex::new(0),
            vec![],
        ))),
        SignatureToken::U64,
        SignatureToken::MutableReference(Box::new(SignatureToken::U64)),
        SignatureToken::Reference(Box::new(SignatureToken::Struct(
            StructHandleIndex::new(0),
            vec![],
        ))),
        SignatureToken::Reference(Box::new(SignatureToken::U64)),
        SignatureToken::MutableReference(Box::new(SignatureToken::Struct(
            StructHandleIndex::new(0),
            vec![],
        ))),
        SignatureToken::Reference(Box::new(SignatureToken::Struct(
            StructHandleIndex::new(0),
            vec![],
        ))),
        SignatureToken::Reference(Box::new(SignatureToken::Struct(
            StructHandleIndex::new(0),
            vec![],
        ))),
        SignatureToken::Reference(Box::new(SignatureToken::U64)),
        SignatureToken::U64,
    ];
    assert_eq!(actual_code, expected_code);
    assert_eq!(actual_types, expected_types);
}

#[test]
fn transform_code_with_arithmetic_ops() {
    let code = String::from(
        "
        module Foobar {

            public arithmetic_ops(a: u64, b: u64): u64 * u64 {
                let c: u64;
                c = (copy(a) + move(b) - 1) * 2 / 3 % 4 | 5 & 6 ^ 7;
                return move(c), move(a);
            }
        }
        ",
    );

    let (actual_code, actual_types) = generate_code_from_string(code);
    let expected_code = vec![
        CopyLoc(3, 0),
        MoveLoc(4, 1),
        Add(5, 3, 4),
        LdConst(6, 1),
        Sub(7, 5, 6),
        LdConst(8, 2),
        Mul(9, 7, 8),
        LdConst(10, 3),
        Div(11, 9, 10),
        LdConst(12, 4),
        Mod(13, 11, 12),
        LdConst(14, 5),
        LdConst(15, 6),
        BitAnd(16, 14, 15),
        BitOr(17, 13, 16),
        LdConst(18, 7),
        Xor(19, 17, 18),
        StLoc(2, 19),
        MoveLoc(20, 2),
        MoveLoc(21, 0),
        Ret(vec![20, 21]),
    ];
    assert_eq!(actual_types.len(), 22);
    for actual_type in actual_types {
        assert_eq!(actual_type, SignatureToken::U64);
    }
    assert_eq!(actual_code, expected_code);
}

#[test]
fn transform_code_with_pack_unpack() {
    let code = String::from(
        "
        module Foobar {
            resource T { x: u64, y: address }

            public pack_unpack(a: address) {
                let t: R#Self.T;
                let x_d: u64;
                let y_d: address;

                t = T { x: 42, y: move(a) };
                T { x_d, y_d } = move(t);
                return;
            }
        }
        ",
    );
    let (actual_code, actual_types) = generate_code_from_string(code);
    let expected_code = vec![
        LdConst(4, 42),
        MoveLoc(5, 0),
        Pack(6, StructDefinitionIndex::new(0), vec![4, 5]),
        StLoc(1, 6),
        MoveLoc(7, 1),
        Unpack(vec![8, 9], StructDefinitionIndex::new(0), 7),
        StLoc(3, 9),
        StLoc(2, 8),
        Ret(vec![]),
    ];
    let expected_types = vec![
        SignatureToken::Address,
        SignatureToken::Struct(StructHandleIndex::new(0), vec![]),
        SignatureToken::U64,
        SignatureToken::Address,
        SignatureToken::U64,
        SignatureToken::Address,
        SignatureToken::Struct(StructHandleIndex::new(0), vec![]),
        SignatureToken::Struct(StructHandleIndex::new(0), vec![]),
        SignatureToken::U64,
        SignatureToken::Address,
    ];
    assert_eq!(actual_code, expected_code);
    assert_eq!(actual_types, expected_types);
}

#[test]
fn transform_code_with_ld_instrs() {
    let code = String::from(
        "
        module Foobar {

            public load() {
                let a: bytearray;
                let b: bool;
                let c: address;
                a = h\"deadbeef\";
                b = true;
                b = false;
                c = 0xdeadbeef;
                return;
            }
        }
        ",
    );
    let (actual_code, actual_types) = generate_code_from_string(code);
    let expected_code = vec![
        LdByteArray(3, ByteArrayPoolIndex::new(0)),
        StLoc(0, 3),
        LdTrue(4),
        StLoc(1, 4),
        LdFalse(5),
        StLoc(1, 5),
        LdAddr(6, AddressPoolIndex::new(1)),
        StLoc(2, 6),
        Ret(vec![]),
    ];
    let expected_types = vec![
        SignatureToken::ByteArray,
        SignatureToken::Bool,
        SignatureToken::Address,
        SignatureToken::ByteArray,
        SignatureToken::Bool,
        SignatureToken::Bool,
        SignatureToken::Address,
    ];
    assert_eq!(actual_code, expected_code);
    assert_eq!(actual_types, expected_types);
}

#[test]
fn transform_code_with_easy_branching() {
    let code = String::from(
        "
        module Foobar {

            public branching() {
                loop {
                    if (true) {
                        break;
                    } else {
                        continue;
                    }
                }
                assert(!false, 42);
                return;
            }
        }
        ",
    );
    let (actual_code, actual_types) = generate_code_from_string(code);
    let expected_code = vec![
        LdTrue(0),
        BrFalse(10, 0),
        Branch(3),
        LdFalse(1),
        Not(2, 1),
        Not(3, 2),
        BrFalse(9, 3),
        LdConst(4, 42),
        Abort(4),
        Ret(vec![]),
        Branch(0),
    ];
    let expected_types = vec![
        SignatureToken::Bool,
        SignatureToken::Bool,
        SignatureToken::Bool,
        SignatureToken::Bool,
        SignatureToken::U64,
    ];
    assert_eq!(actual_code, expected_code);
    assert_eq!(actual_types, expected_types);
}

#[test]
fn transform_code_with_bool_ops() {
    let code = String::from(
        "
        module Foobar {

            public bool_ops(a: u64, b: u64) {
                let c: bool;
                let d: bool;
                c = (copy(a) > copy(b)) && (copy(a) >= copy(b));
                d = (copy(a) < copy(b)) || (copy(a) <= copy(b));
                assert(!(move(c) != move(d)), 42);
                return;
            }
        }
        ",
    );
    let (actual_code, actual_types) = generate_code_from_string(code);
    let expected_code = vec![
        CopyLoc(4, 0),
        CopyLoc(5, 1),
        Gt(6, 4, 5),
        CopyLoc(7, 0),
        CopyLoc(8, 1),
        Ge(9, 7, 8),
        And(10, 6, 9),
        StLoc(2, 10),
        CopyLoc(11, 0),
        CopyLoc(12, 1),
        Lt(13, 11, 12),
        CopyLoc(14, 0),
        CopyLoc(15, 1),
        Le(16, 14, 15),
        Or(17, 13, 16),
        StLoc(3, 17),
        MoveLoc(18, 2),
        MoveLoc(19, 3),
        Neq(20, 18, 19),
        Not(21, 20),
        Not(22, 21),
        BrFalse(24, 22),
        LdConst(23, 42),
        Abort(23),
        Ret(vec![]),
    ];
    let expected_types = vec![
        SignatureToken::U64,
        SignatureToken::U64,
        SignatureToken::Bool,
        SignatureToken::Bool,
        SignatureToken::U64,
        SignatureToken::U64,
        SignatureToken::Bool,
        SignatureToken::U64,
        SignatureToken::U64,
        SignatureToken::Bool,
        SignatureToken::Bool,
        SignatureToken::U64,
        SignatureToken::U64,
        SignatureToken::Bool,
        SignatureToken::U64,
        SignatureToken::U64,
        SignatureToken::Bool,
        SignatureToken::Bool,
        SignatureToken::Bool,
        SignatureToken::Bool,
        SignatureToken::Bool,
        SignatureToken::Bool,
        SignatureToken::Bool,
        SignatureToken::U64,
    ];
    assert_eq!(actual_code, expected_code);
    assert_eq!(actual_types, expected_types);
}

#[test]
fn transform_code_with_txn_builtins() {
    let code = String::from(
        "
        module Foobar {

            public txn_builtins() {
                let addr: address;
                let seq_num: u64;
                let max_gas: u64;
                let gas_price: u64;
                let gas: u64;
                let pk: bytearray;
                gas = get_gas_remaining();
                seq_num = get_txn_sequence_number();
                max_gas = get_txn_max_gas_units();
                gas_price = get_txn_gas_unit_price();
                addr = get_txn_sender();
                pk = get_txn_public_key();
                create_account(move(addr));
                return;
            }
        }
        ",
    );
    let (actual_code, actual_types) = generate_code_from_string(code);
    let expected_code = vec![
        GetGasRemaining(6),
        StLoc(4, 6),
        GetTxnSequenceNumber(7),
        StLoc(1, 7),
        GetTxnMaxGasUnits(8),
        StLoc(2, 8),
        GetTxnGasUnitPrice(9),
        StLoc(3, 9),
        GetTxnSenderAddress(10),
        StLoc(0, 10),
        GetTxnPublicKey(11),
        StLoc(5, 11),
        MoveLoc(12, 0),
        CreateAccount(12),
        Ret(vec![]),
    ];
    let expected_types = vec![
        SignatureToken::Address,
        SignatureToken::U64,
        SignatureToken::U64,
        SignatureToken::U64,
        SignatureToken::U64,
        SignatureToken::ByteArray,
        SignatureToken::U64,
        SignatureToken::U64,
        SignatureToken::U64,
        SignatureToken::U64,
        SignatureToken::Address,
        SignatureToken::ByteArray,
        SignatureToken::Address,
    ];
    assert_eq!(actual_code, expected_code);
    assert_eq!(actual_types, expected_types);
}

#[test]
fn transform_code_with_function_call() {
    let code = String::from(
        "
        module Foobar {

            public foo(aa: address, bb: u64, cc: bytearray) {
                let a: address;
                let b: u64;
                let c: bytearray;
                a,b,c = Self.bar(move(cc),move(aa),move(bb));
                return;
            }

            public bar(c: bytearray, a: address, b:u64): address*u64*bytearray {
                return move(a), move(b), move(c);
            }
        }
        ",
    );
    let (actual_code, actual_types) = generate_code_from_string(code);
    let expected_code = vec![
        MoveLoc(6, 2),
        MoveLoc(7, 0),
        MoveLoc(8, 1),
        Call(vec![11, 10, 9], FunctionHandleIndex::new(1), vec![6, 7, 8]),
        StLoc(5, 11),
        StLoc(4, 10),
        StLoc(3, 9),
        Ret(vec![]),
    ];
    let expected_types = vec![
        SignatureToken::Address,
        SignatureToken::U64,
        SignatureToken::ByteArray,
        SignatureToken::Address,
        SignatureToken::U64,
        SignatureToken::ByteArray,
        SignatureToken::ByteArray,
        SignatureToken::Address,
        SignatureToken::U64,
        SignatureToken::Address,
        SignatureToken::U64,
        SignatureToken::ByteArray,
    ];
    assert_eq!(actual_code, expected_code);
    assert_eq!(actual_types, expected_types);
}

#[test]
fn transform_code_with_module_builtins() {
    let code = String::from(
        "
        module Foobar {
            resource T {
                x: u64,
            }

            public module_builtins(a: address):  &mut R#Self.T {
                let t: R#Self.T;
                let t_ref: &mut R#Self.T;
                let b: bool;

                b = exists<T>(copy(a));
                t_ref = borrow_global<T>(copy(a));
                t = move_from<T>(copy(a));
                move_to_sender<T>(move(t));
                return move(t_ref);
            }
        }
        ",
    );
    let (actual_code, actual_types) = generate_code_from_string(code);
    let expected_code = vec![
        CopyLoc(4, 0),
        Exists(5, 4, StructDefinitionIndex::new(0)),
        StLoc(3, 5),
        CopyLoc(6, 0),
        BorrowGlobal(7, 6, StructDefinitionIndex::new(0)),
        StLoc(2, 7),
        CopyLoc(8, 0),
        MoveFrom(9, 8, StructDefinitionIndex::new(0)),
        StLoc(1, 9),
        MoveLoc(10, 1),
        MoveToSender(10, StructDefinitionIndex::new(0)),
        MoveLoc(11, 2),
        Ret(vec![11]),
    ];
    let expected_types = vec![
        SignatureToken::Address,
        SignatureToken::Struct(StructHandleIndex::new(0), vec![]),
        SignatureToken::MutableReference(Box::new(SignatureToken::Struct(
            StructHandleIndex::new(0),
            vec![],
        ))),
        SignatureToken::Bool,
        SignatureToken::Address,
        SignatureToken::Bool,
        SignatureToken::Address,
        SignatureToken::MutableReference(Box::new(SignatureToken::Struct(
            StructHandleIndex::new(0),
            vec![],
        ))),
        SignatureToken::Address,
        SignatureToken::Struct(StructHandleIndex::new(0), vec![]),
        SignatureToken::Struct(StructHandleIndex::new(0), vec![]),
        SignatureToken::MutableReference(Box::new(SignatureToken::Struct(
            StructHandleIndex::new(0),
            vec![],
        ))),
    ];
    assert_eq!(actual_code, expected_code);
    assert_eq!(actual_types, expected_types);
}

fn generate_code_from_string(code: String) -> (Vec<StacklessBytecode>, Vec<SignatureToken>) {
    let address = &AccountAddress::default();
    let module = parse_module(&code).unwrap();
    let deps: Vec<CompiledModule> = vec![];
    let compiled_module_res = compile_module(&address, &module, &deps).unwrap();
    let res = StacklessModuleGenerator::new(&compiled_module_res).generate_module();
    let code = res[0].code.clone();
    let types = res[0].local_types.clone();
    (code, types)
}
