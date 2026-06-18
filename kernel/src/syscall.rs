use core::arch::global_asm;

#[repr(C)]
pub struct SyscallResult {
    value: i64,
    error: i64,
}

pub type Syscall = extern "C" fn(regs: usize) -> SyscallResult;

global_asm! { r#"
.global _syscall_entry
_syscall_entry:
    push rbp
    push rcx # original RIP
    push r11 # original RFLAGS
    
    mov rbp, rsp

    # r10 is third argument for syscall
    mov rcx, r10
    
    # Adjust stack to 16-byte align
    and rsp, 0xfffffffffffffff0
    
    push rax
    push rdx

    # レジスタの保存
    push rcx
    push rsi
    push rdi
    push r8
    push r9
    push r10

    cli
    call get_current_task_os_stack_pointer
    sti

    # レジスタの復帰
    pop r10
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rcx

    mov rdx, [rsp + 0] # RDX
    mov [rax - 16], rdx
    mov rdx, [rsp + 8] # rax
    mov [rax - 8], rdx

    lea rsp, [rax - 16]
    pop rdx
    pop rax
    and rsp, 0xfffffffffffffff0

    call [SYSCALL_TABLE + 8 * eax]
    # rbx, r12-r15 は callee-saved なので呼び出し側では保存しない
    # rax は戻り値用なので呼び出し側では保存しない

    mov rsp, rbp

    pop rsi # システムコール番号の復帰

    # 0x8000_0002 の場合は exit 処理
    cmp esi, 0x80000002
    je .exit

    pop r11
    pop rcx
    pop rbp

    sysretq

.exit:
    mov rsp, rax # RSP
    mov eax, edx # exit() の引数

    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx

    ret # call_app の次の行に飛ぶ

.global exit_app_unsafe     # exit_app_unsafe(rsp: u64, ret_val: i32)
exit_app_unsafe:
    mov rsp, rdi
    mov eax, esi

    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx

    ret # call_app の次の行に飛ぶ
"# }
