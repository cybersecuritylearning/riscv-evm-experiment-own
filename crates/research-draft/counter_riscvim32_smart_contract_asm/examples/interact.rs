//! Deploy the counter, then call it: increment(), getValue(), setValue(x).
//! Run: cargo run -p counter_riscvim32_smart_contract_asm --example interact
use revm::{
    Context as RevmEthContext, DatabaseCommit, MainContext,
    context::{ContextTr, JournalTr},
    database::CacheDB,
    primitives::{Address, Bytes, U256},
};
use riscv_assembler::assembler::assemble;
use riscv_evm::{
    context::Context,
    ecall_manager::process_ecall,
    riscv_evm_core::{MemoryChuckSize, e_constants::*, interfaces::MemoryInterface},
    utils::{u32_vec_to_address, u32_vec_to_bytes},
    vm::Vm,
};

#[path = "../src/contract.rs"]
mod contract;

/// With `--fix`, patch the two memory-address bugs in the sample contract (in memory only, contract.rs is untouched).
fn source() -> String {
    let mut src = contract::ASSEMBLE_CODE.to_string();
    if std::env::args().any(|a| a == "--fix") {
        src = src
            .replace("addi x2, zero, -48           # Allocate stack space", "addi x2, zero, 1024          # scratch memory")
            .replace("addi x2, x2, -8           # Allocate stack space for uint256", "addi x2, zero, 1024         # scratch memory")
            .replace("addi x2, zero, 8                # length = 8 bytes (uint256)", "addi x2, zero, 32               # length = 32 bytes (uint256)");
    }
    src
}

fn deploy(context: &mut Context) -> Address {
    let mut vm = Vm::new();
    let words = assemble(&source()).unwrap().code;
    let init_code = u32_vec_to_bytes(&words, words.len() * 4);
    for (i, &b) in init_code.iter().enumerate() {
        vm.memory.write_mem(900 + i as u32, MemoryChuckSize::BYTE, b as u32);
    }
    vm.registers.write_reg(ECALL_CODE_REG, 0xF0);
    vm.registers.write_reg(CREATE_INPUT_REGISTER_1, 900);
    vm.registers.write_reg(CREATE_INPUT_REGISTER_2, init_code.len() as u32);
    context.eth_context.journal().load_account(context.address).unwrap();
    let result = process_ecall(&mut vm, context).unwrap();
    for r in result.into_iter().rev() {
        context.eth_context.db().commit(r.state);
    }
    let regs: Vec<u32> = (0..5).map(|i| vm.registers.read_reg(CREATE_OUTPUT_REGISTER_1 + i)).collect();
    Address::from(u32_vec_to_address(&regs))
}

/// Run the runtime code of `contract` with `calldata` and commit state changes.
fn call(context: &mut Context, contract: Address, calldata: Vec<u8>) -> Bytes {
    let code = context
        .eth_context
        .journal()
        .load_account_code(contract)
        .unwrap()
        .info
        .code
        .clone()
        .unwrap()
        .original_bytes();
    let mut vm = Vm::from_bin_u8(code.to_vec()).unwrap();
    context.address = contract;
    context.eth_context.tx.data = calldata.into();
    context.return_data = Bytes::new();
    context.eth_context.journal().load_account(contract).unwrap();
    context.eth_context.journal().touch_account(contract); // mark touched so CacheDB::commit keeps storage changes
    vm.run(false, context);
    let state = context.eth_context.journal().finalize();
    context.eth_context.db().commit(state.state);
    context.return_data.clone()
}

fn slot0(context: &mut Context, c: Address) -> U256 {
    context
        .eth_context
        .db()
        .cache
        .accounts
        .get(&c)
        .and_then(|a| a.storage.get(&U256::ZERO).copied())
        .unwrap_or_default()
}

fn selector(n: u32) -> Vec<u8> {
    n.to_be_bytes().to_vec()
}

fn main() {
    let eth_context = RevmEthContext::mainnet().with_db(CacheDB::default());
    let mut context = Context::new(eth_context);
    context.current_caller = Address::from([0xAA; 20]);

    let c = deploy(&mut context);
    println!("deployed at {c:?}");
    println!("slot0 after deploy      = {}", slot0(&mut context, c));

    for (name, data) in [
        ("increment()", selector(0x37)),
        ("increment()", selector(0x37)),
        ("getValue()", selector(0x20)),
        ("setValue(42)", {
            let mut d = selector(0x55);
            d.extend_from_slice(&U256::from(42).to_be_bytes::<32>());
            d
        }),
        ("getValue()", selector(0x20)),
        ("unknown selector", selector(0x99)),
    ] {
        let ret = call(&mut context, c, data);
        println!("{name:18} -> return={ret:?}  slot0={}", slot0(&mut context, c));
    }
}