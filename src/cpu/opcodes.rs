/// CPU 操作碼枚舉
#[derive(Debug, Clone, Copy)]
pub enum Opcode {
    // 8位載入指令
    LdRegReg(Register8, Register8), // LD r,r
    LdRegImm(Register8, u8),        // LD r,n
    LdRegHl(Register8),             // LD r,(HL)
    LdHlReg(Register8),             // LD (HL),r
    LdHlImm(u8),                    // LD (HL),n
    LdABc,                          // LD A,(BC)
    LdADe,                          // LD A,(DE)
    LdAAddr(u16),                   // LD A,(nn)
    LdBcA,                          // LD (BC),A
    LdDeA,                          // LD (DE),A
    LdAddrA(u16),                   // LD (nn),A
    LdhAImm(u8),                    // LDH A,(n)
    LdhImmA(u8),                    // LDH (n),A
    LdhAC,                          // LD A,(FF00+C)
    LdhCA,                          // LD (FF00+C),A

    // 16位載入指令    LdRr16Imm(Register16, u16),        // LD rr,nn
    LdAddrSp(u16),      // LD (nn),SP
    LdSpHl,             // LD SP,HL
    PushRr(Register16), // PUSH rr
    PopRr(Register16),  // POP rr

    // 8位算術/邏輯指令
    AddAR(Register8), // ADD A,r
    AddAImm(u8),      // ADD A,n
    AddAHl,           // ADD A,(HL)
    AdcAR(Register8), // ADC A,r
    AdcAImm(u8),      // ADC A,n
    AdcAHl,           // ADC A,(HL)
    SubR(Register8),  // SUB r
    SubImm(u8),       // SUB n
    SubHl,            // SUB (HL)
    SbcAR(Register8), // SBC A,r
    SbcAImm(u8),      // SBC A,n
    SbcAHl,           // SBC A,(HL)
    AndR(Register8),  // AND r
    AndImm(u8),       // AND n
    AndHl,            // AND (HL)
    OrR(Register8),   // OR r
    OrImm(u8),        // OR n
    OrHl,             // OR (HL)
    XorR(Register8),  // XOR r
    XorImm(u8),       // XOR n
    XorHl,            // XOR (HL)
    CpR(Register8),   // CP r
    CpImm(u8),        // CP n
    CpHl,             // CP (HL)
    IncR(Register8),  // INC r
    IncHlMem,         // INC (HL)
    DecR(Register8),  // DEC r
    DecHlMem,         // DEC (HL)

    // 16位算術指令    AddHlRr(Register16),               // ADD HL,rr
    AddSpImm(i8),      // ADD SP,n
    IncRr(Register16), // INC rr
    DecRr(Register16), // DEC rr

    // 旋轉/位移指令
    Rlca,            // RLCA
    Rla,             // RLA
    Rrca,            // RRCA
    Rra,             // RRA
    RlcR(Register8), // RLC r
    RlcHl,           // RLC (HL)
    RlR(Register8),  // RL r
    RlHl,            // RL (HL)
    RrcR(Register8), // RRC r
    RrcHl,           // RRC (HL)
    RrR(Register8),  // RR r
    RrHl,            // RR (HL)
    SlaR(Register8), // SLA r
    SlaHl,           // SLA (HL)
    SraR(Register8), // SRA r
    SraHl,           // SRA (HL)
    SrlR(Register8), // SRL r
    SrlHl,           // SRL (HL)

    // 位操作指令
    BitNR(u8, Register8),     // BIT n,r
    BitNHl(u8),               // BIT n,(HL)
    SetNR(u8, Register8),     // SET n,r
    SetNHl(u8),               // SET n,(HL)
    ResNR(u8, Register8),     // RES n,r
    ResNHl(u8),               // RES n,(HL)    // 跳轉指令
    JpNn(u16),                // JP nn
    JpCcNn(Condition, u16),   // JP cc,nn
    JpHl,                     // JP (HL)
    JrN(i8),                  // JR n
    JrCcN(Condition, i8),     // JR cc,n
    CallNn(u16),              // CALL nn
    CallCcNn(Condition, u16), // CALL cc,nn
    Ret,                      // RET
    RetCc(Condition),         // RET cc
    Reti,                     // RETI
    RstN(u8),                 // RST n

    // CPU 控制指令
    Ccf,         // CCF
    Scf,         // SCF
    Nop,         // NOP
    Halt,        // HALT
    Stop,        // STOP
    Di,          // DI
    Ei,          // EI
    PrefixCb,    // CB prefix
    Invalid(u8), // Invalid opcode
}

/// CPU 8位寄存器
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register8 {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

/// CPU 16位寄存器
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register16 {
    AF,
    BC,
    DE,
    HL,
    SP,
}

/// 條件碼
#[derive(Debug, Clone, Copy)]
pub enum Condition {
    NZ, // 非零
    Z,  // 零
    NC, // 無進位
    C,  // 進位
}
