# RISC-V EVM Experiment: Project Overview

A walkthrough of the counter example, what it does, and what is left to fix. Based on running the project, October 27, 2026.

## What the project is

A research prototype of a **RISC-V virtual machine that executes smart contracts inside an Ethereum (REVM) host**. Instead of contracts written in EVM bytecode, they are written in RISC-V (RV32IM) machine code. The VM talks to Ethereum state through **ecalls**, RISC-V's system-call instruction. Each ecall plays the role of an EVM opcode: `SLOAD`, `SSTORE`, `CALLER`, `CALL`, `CREATE`, `RETURN`, `LOG` and so on.

It is the groundwork for the idea of running things on RISC-V while staying compatible with Ethereum.

## The pieces

| Piece | Role |
| --- | --- |
| `riscv_evm_core` | Registers, memory, and the ecall numbers and register conventions (`e_constants.rs`) |
| `riscv_evm` | The VM: instruction decoding and execution (`vm.rs`), the ecall handlers that bridge to REVM (`ecall_manager.rs`), and an ELF loader |
| `riscv_assembler` (external crate) | Turns assembly text into 32-bit instruction words |
| `counter_riscvim32_smart_contract_asm` | The sample contract (`contract.rs`) and the deploy example |
| `riscv_smart_contracts` | More contracts, including an ERC20 in assembly |
| `research-final` | The in-progress REVM handler and benchmarks |

## How the example works, end to end

1. **Deploy.** The assembly text is assembled into words, converted to bytes, and copied into a VM's memory. The Create ecall derives an address, runs the init code in a new VM, takes the runtime code it returns, and stores it on the new account.
2. **Call.** The stored runtime code is loaded into a fresh VM and the calldata is placed in `tx.data`. The code reads the selector with `CALLDATALOAD` and jumps to `increment`, `getValue` or `setValue`. Storage goes through `SLOAD` and `SSTORE`, and results go out through `RETURN`.

The contract uses a convention of 256-bit values spread across eight 32-bit registers: `x1` to `x8` for keys, `x9` to `x16` for values.

## What you have now

- A working deploy and call loop (`interact.rs`), with the counter going 1, 2, 3, then 42 and returning the right values after the fixes.
- A clear list of fixes needed before the project is solid:
  - the return-path addresses in the contract (now fixed)
  - the commit order in `main.rs`, which loses the deployed code
  - the value transfer that fails silently
  - the silent 100-step cap
  - errors during init code being swallowed
  - no gas metering and a small fixed memory

## What it is not yet

It is not a full EVM-compatible RISC-V chain. There is no gas accounting, no legacy EVM bytecode path (the compatibility layer needed for the goal of moving everything onto RISC-V), and nested calls are limited. Those are the gaps between this prototype and the target design.

## Next step

Build a second contract. Options, from simplest: a small registry keyed by caller, a contract that emits logs, or the ERC20 already in the repo (`erc20.s`).
