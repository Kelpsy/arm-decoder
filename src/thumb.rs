use crate::{ArmVersion, Processor};
use core::mem::{transmute, MaybeUninit};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShiftImmType {
    Lsl,
    Lsr,
    Asr,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DpOpImm8Type {
    Mov,
    Cmp,
    Add,
    Sub,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DpOpRegType {
    And,
    Eor,
    Lsl,
    Lsr,
    Asr,
    Adc,
    Sbc,
    Ror,
    Tst,
    Neg,
    Cmp,
    Cmn,
    Orr,
    Mul,
    Bic,
    Mvn,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DpOpSpecialType {
    Add,
    Cmp,
    Mov,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LoadStoreRegTy {
    Str,
    Strh,
    Strb,
    Ldrsb,
    Ldr,
    Ldrh,
    Ldrb,
    Ldrsh,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RevType {
    Rev,
    Rev16,
    Revsh,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Instr {
    AddSubRegImm3 {
        sub: bool,
        imm3: bool,
        op: u8,
    },
    ShiftImm {
        ty: ShiftImmType,
        shift: u8,
    },
    DpOpImm8 {
        ty: DpOpImm8Type,
        reg: u8,
    },
    DpOpReg(DpOpRegType),
    Bx {
        link: bool,
        addr_high_reg: bool,
    },
    DpOpSpecial {
        ty: DpOpSpecialType,
        h1: bool,
        h2: bool,
    },
    LoadPcRel {
        reg: u8,
    },
    LoadStoreReg {
        ty: LoadStoreRegTy,
        offset_reg: u8,
    },
    LoadStoreWbImm {
        byte: bool,
        load: bool,
        offset: u8,
    },
    LoadStoreHalfImm {
        load: bool,
        offset: u8,
    },
    LoadStoreStack {
        load: bool,
        reg: u8,
    },
    AddSpPcImm {
        sp: bool,
        dst_reg: u8,
    },
    AddSubSpImm7 {
        sub: bool,
    },
    Extend {
        unsigned: bool,
        byte: bool,
    },
    PushPop {
        pop: bool,
        push_r14_pop_r15: bool,
    },
    CpsSetEnd,
    Rev(RevType),
    Bkpt,
    LoadStoreMultiple {
        load: bool,
        base_reg: u8,
    },
    Swi,
    CondBranch {
        cond: u8,
    },
    Branch,
    BlPrefix,
    BlSuffix {
        exchange: bool,
    },
    Undefined {
        stable: bool,
    },
}

#[allow(clippy::too_many_lines)]
const fn decode_thumb(instr: u16, proc: Processor) -> Instr {
    // Add/sub reg/imm3 instructions use a hole in the lsl/lsr/asr imm encoding and must go before
    if instr & 0xF800 == 0x1800 {
        // Add/sub reg/imm3
        Instr::AddSubRegImm3 {
            sub: instr & 1 << 9 != 0,
            imm3: instr & 1 << 10 != 0,
            op: (instr >> 6 & 7) as u8,
        }
    } else if instr & 0xE000 == 0 {
        // Shift by immediate
        Instr::ShiftImm {
            ty: unsafe { core::mem::transmute((instr >> 11 & 3) as u8) },
            shift: (instr >> 6 & 0x1F) as u8,
        }
    } else if instr & 0xE000 == 0x2000 {
        // Add/sub/cmp/mov imm8
        Instr::DpOpImm8 {
            ty: unsafe { core::mem::transmute((instr >> 11 & 3) as u8) },
            reg: (instr >> 8 & 7) as u8,
        }
    } else if instr & 0xFC00 == 0x4000 {
        // Data processing register
        Instr::DpOpReg(unsafe { core::mem::transmute((instr >> 6 & 0xF) as u8) })
    // Bx/blx must go before special/high-reg data processing as it uses a hole in the encoding
    } else if instr & 0xFF00 == 0x4700 {
        // Bx/blx
        // TODO: What happens when using BLX on processors before ARMv5T?
        Instr::Bx {
            link: instr & 1 << 7 != 0 && proc.has_thumb_blx(),
            addr_high_reg: instr & 1 << 6 != 0,
        }
    } else if instr & 0xFC00 == 0x4400 {
        // Special data processing
        Instr::DpOpSpecial {
            ty: unsafe { core::mem::transmute((instr >> 8 & 3) as u8) },
            h1: instr & 1 << 7 != 0,
            h2: instr & 1 << 6 != 0,
        }
    } else if instr & 0xF800 == 0x4800 {
        // Load from literal pool
        Instr::LoadPcRel {
            reg: (instr >> 8 & 7) as u8,
        }
    } else if instr & 0xF000 == 0x5000 {
        // Load/store register offset
        Instr::LoadStoreReg {
            ty: unsafe { core::mem::transmute((instr >> 9 & 7) as u8) },
            offset_reg: (instr >> 6 & 7) as u8,
        }
    } else if instr & 0xE000 == 0x6000 {
        // Load/store word/byte imm offset
        Instr::LoadStoreWbImm {
            byte: instr & 1 << 12 != 0,
            load: instr & 1 << 11 != 0,
            offset: (instr >> 6 & 0x1F) as u8,
        }
    } else if instr & 0xF000 == 0x8000 {
        // Load/store half imm offset
        Instr::LoadStoreHalfImm {
            load: instr & 1 << 11 != 0,
            offset: (instr >> 6 & 0x1F) as u8,
        }
    } else if instr & 0xF000 == 0x9000 {
        // Load/store to/from stack
        Instr::LoadStoreStack {
            load: instr & 1 << 11 != 0,
            reg: (instr >> 8 & 7) as u8,
        }
    } else if instr & 0xF000 == 0xA000 {
        // Add to SP or PC
        Instr::AddSpPcImm {
            sp: instr & 1 << 11 != 0,
            dst_reg: (instr >> 8 & 7) as u8,
        }
    } else if instr & 0xF000 == 0xB000 {
        // Miscellaneous
        match instr >> 8 & 0xF {
            0 => Instr::AddSubSpImm7 {
                sub: instr & 1 << 7 != 0,
            },
            2 if proc.arm_version().is_at_least(ArmVersion::V6) => Instr::Extend {
                unsigned: instr & 1 << 7 != 0,
                byte: instr & 1 << 6 != 0,
            },
            4 | 5 | 0xC | 0xD => Instr::PushPop {
                pop: instr & 1 << 11 != 0,
                push_r14_pop_r15: instr & 1 << 8 != 0,
            },
            6 if instr & 0x00C0 == 0x0040 && proc.arm_version().is_at_least(ArmVersion::V6) => {
                Instr::CpsSetEnd
            }
            0xA if proc.arm_version().is_at_least(ArmVersion::V6) => {
                Instr::Rev(match instr >> 6 & 3 {
                    0 => RevType::Rev,
                    1 => RevType::Rev16,
                    3 => RevType::Revsh,
                    _ => return Instr::Undefined { stable: false },
                })
            }
            0xE if proc.arm_version().is_at_least(ArmVersion::V5) => Instr::Bkpt,
            _ => Instr::Undefined { stable: false },
        }
    } else if instr & 0xF000 == 0xC000 {
        // Load/store multiple
        Instr::LoadStoreMultiple {
            load: instr & 1 << 11 != 0,
            base_reg: (instr >> 8 & 7) as u8,
        }
    // The undefined and swi instructions use holes in the cond branch encoding
    } else if instr & 0xFF00 == 0xDE00 {
        // (Architecturally) undefined instruction
        Instr::Undefined { stable: true }
    } else if instr & 0xFF00 == 0xDF00 {
        // Software interrupt
        Instr::Swi
    } else if instr & 0xF000 == 0xD000 {
        // Conditional branch
        Instr::CondBranch {
            cond: (instr >> 8 & 0xF) as u8,
        }
    } else if instr & 0xF800 == 0xE000 {
        // Unconditional branch
        Instr::Branch
    } else if instr & 0xF800 == 0xE800 && proc.arm_version().is_at_least(ArmVersion::V5) {
        // BLX suffix (or undefined)
        Instr::BlSuffix { exchange: true }
    } else if instr & 0xF800 == 0xF000 {
        // BL/BLX prefix
        Instr::BlPrefix
    } else if instr & 0xF800 == 0xF800 {
        // BL suffix
        Instr::BlSuffix { exchange: false }
    } else {
        Instr::Undefined { stable: false }
    }
}

#[must_use]
pub fn thumb(proc: Processor) -> [Instr; 0x400] {
    // The upper 10 bits are used for decoding
    let mut instr_table = [MaybeUninit::uninit(); 0x400];
    for (i, instr) in instr_table.iter_mut().enumerate() {
        *instr = MaybeUninit::new(decode_thumb((i << 6) as u16, proc));
    }
    unsafe { transmute(instr_table) }
}
