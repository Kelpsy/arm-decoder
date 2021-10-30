use crate::{ArmVersion, Processor};
use core::mem::{transmute, MaybeUninit};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DpOpTy {
    And,
    Eor,
    Sub,
    Rsb,
    Add,
    Adc,
    Sbc,
    Rsc,
    Tst,
    Teq,
    Cmp,
    Cmn,
    Orr,
    Mov,
    Bic,
    Mvn,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MsrTy {
    Reg,
    Imm,
    ImmMaybeHint,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DspMulTy {
    Smulxy { acc: bool },
    Smulwy { acc: bool },
    Smlalxy,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShiftTy {
    Lsl,
    Lsr,
    Asr,
    Ror,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DpOperand {
    Imm,
    Reg { shift_ty: ShiftTy, shift_imm: bool },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransferSize {
    Byte,
    Half,
    Word,
    Dword,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MiscTransferTy {
    Ldrh,
    Strh,
    Ldrd,
    Strd,
    Ldrsb,
    Ldrsh,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MiscAddressing {
    Post,
    PreNoWb,
    Pre,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WbAddressing {
    Post,
    PostUser,
    PreNoWb,
    Pre,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WbOff {
    Imm,
    Reg(ShiftTy),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParAddSubPrefix {
    S,
    Q,
    Sh,
    U,
    Uq,
    Uh,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParAddSubTy {
    Add16,
    AddSubX,
    SubAddX,
    Sub16,
    Add8,
    Sub8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExtSrc {
    ParBytes,
    Half,
    Byte,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MediaMulTy {
    Smlad,
    Smlsd,
    Smlald,
    Smlsld,
    Smmla,
    Smmls,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub enum Instr {
    Nop,
    Mrs {
        spsr: bool,
    },
    Msr {
        ty: MsrTy,
        spsr: bool,
    },
    Bx {
        link: bool,
    },
    Bxj,
    Clz,
    SatAddSub {
        sub: bool,
        doubled: bool,
    },
    Bkpt,
    DspMul(DspMulTy),
    DpOp {
        ty: DpOpTy,
        set_flags: bool,
        op: DpOperand,
    },
    Mul {
        acc: bool,
        set_flags: bool,
    },
    MulLong {
        acc: bool,
        set_flags: bool,
        signed: bool,
    },
    Umaal,
    Swp {
        byte: bool,
    },
    TransferExclusive {
        size: TransferSize,
        load: bool,
    },
    MiscTransfer {
        ty: MiscTransferTy,
        addressing: MiscAddressing,
        offset_upwards: bool,
        offset_imm: bool,
    },
    WbTransfer {
        load: bool,
        byte: bool,
        addressing: WbAddressing,
        offset_upwards: bool,
        offset: WbOff,
    },
    TransferMultiple {
        load: bool,
        increment: bool,
        base_excluded: bool,
        writeback: bool,
        s_bit: bool,
    },
    ParAddSub {
        prefix: ParAddSubPrefix,
        ty: ParAddSubTy,
    },
    Pkhbt,
    Pkhtb,
    Sat {
        par_half: bool,
        signed: bool,
    },
    Rev,
    Rev16,
    Revsh,
    Sel,
    MediaMul(MediaMulTy),
    Usad8,
    Ext {
        src: ExtSrc,
        signed: bool,
    },
    Branch {
        link: bool,
    },
    Mcrr,
    Mrrc,
    Stc,
    Ldc,
    Cdp,
    Mcr,
    Mrc,
    Swi,
    Undefined {
        stable: bool,
    },
}

#[inline]
const fn dp_op_ty(instr: u32) -> DpOpTy {
    unsafe { core::mem::transmute((instr >> 21 & 0xF) as u8) }
}

#[inline]
const fn shift_ty(instr: u32) -> ShiftTy {
    unsafe { core::mem::transmute((instr >> 5 & 3) as u8) }
}

#[inline]
const fn wb_addressing(instr: u32) -> WbAddressing {
    match (instr & 1 << 24 != 0, instr & 1 << 21 != 0) {
        (false, false) => WbAddressing::Post,
        (false, true) => WbAddressing::PostUser,
        (true, false) => WbAddressing::PreNoWb,
        (true, true) => WbAddressing::Pre,
    }
}

#[allow(clippy::too_many_lines)]
const fn decode_arm_cond(instr: u32, proc: Processor) -> Instr {
    // Control and DSP instructions use a hole in the data processing encoding and must go before
    // DP cases
    if instr & 0x0F90_0010 == 0x0100_0000 || instr & 0x0F90_0090 == 0x0100_0010 {
        // Control and DSP instruction extension space (excluding msr imm)
        match instr >> 4 & 0xF {
            0 => {
                let spsr = instr & 1 << 22 != 0;
                if instr & 1 << 21 == 0 {
                    Instr::Mrs { spsr }
                } else {
                    Instr::Msr {
                        spsr,
                        ty: MsrTy::Reg,
                    }
                }
            }
            1 if instr & 1 << 21 != 0 => {
                if instr & 1 << 22 == 0 {
                    if proc.has_thumb() {
                        Instr::Bx { link: false }
                    } else {
                        Instr::Undefined { stable: false }
                    }
                } else if proc.arm_version().is_at_least(ArmVersion::V5) {
                    Instr::Clz
                } else {
                    Instr::Undefined { stable: false }
                }
            }
            2 if instr >> 21 & 3 == 1 && proc.has_jazelle() => Instr::Bxj,
            3 if instr >> 21 & 3 == 1 && proc.arm_version().is_at_least(ArmVersion::V5) => {
                Instr::Bx { link: true }
            }
            5 if proc.has_enhanced_dsp() => Instr::SatAddSub {
                sub: instr & 1 << 21 != 0,
                doubled: instr & 1 << 22 != 0,
            },
            7 if instr >> 21 & 3 == 1 && proc.arm_version().is_at_least(ArmVersion::V5) => {
                Instr::Bkpt
            }
            8 | 0xA | 0xC | 0xE => {
                if proc.has_enhanced_dsp() {
                    Instr::DspMul(match instr >> 21 & 3 {
                        0 => DspMulTy::Smulxy { acc: true },
                        1 => DspMulTy::Smulwy {
                            acc: instr & 1 << 5 == 0,
                        },
                        2 => DspMulTy::Smlalxy,
                        _ => DspMulTy::Smulxy { acc: false },
                    })
                } else if proc.has_nop_dsp_multiplies() {
                    Instr::Nop
                } else {
                    Instr::Undefined { stable: false }
                }
            }
            _ => Instr::Undefined { stable: false },
        }
    } else if instr & 0x0E00_0010 == 0x0000_0000 {
        // Data processing reg shift imm
        Instr::DpOp {
            ty: dp_op_ty(instr),
            set_flags: instr & 1 << 20 != 0,
            op: DpOperand::Reg {
                shift_ty: shift_ty(instr),
                shift_imm: true,
            },
        }
    } else if instr & 0x0E00_0090 == 0x0000_0010 {
        // Data processing reg shift reg
        Instr::DpOp {
            ty: dp_op_ty(instr),
            set_flags: instr & 1 << 20 != 0,
            op: DpOperand::Reg {
                shift_ty: shift_ty(instr),
                shift_imm: false,
            },
        }
    } else if instr & 0x0E00_0090 == 0x0000_0090 {
        // Multiplies and misc load/stores
        if instr >> 5 & 3 == 0 {
            if instr & 1 << 24 == 0 {
                // Multiply extension space
                let acc = instr & 1 << 21 != 0;
                let set_flags = instr & 1 << 20 != 0;
                match instr >> 22 & 3 {
                    0 => Instr::Mul { acc, set_flags },
                    1 => {
                        if instr >> 20 & 3 == 0 && proc.arm_version().is_at_least(ArmVersion::V6) {
                            Instr::Umaal
                        } else {
                            Instr::Undefined { stable: false }
                        }
                    }
                    2 if proc.has_long_multiply() => Instr::MulLong {
                        acc,
                        set_flags,
                        signed: false,
                    },
                    3 if proc.has_long_multiply() => Instr::MulLong {
                        acc,
                        set_flags,
                        signed: true,
                    },
                    _ => Instr::Undefined { stable: false },
                }
            } else {
                match instr >> 20 & 0xF {
                    0x0 => Instr::Swp { byte: false },
                    0x4 => Instr::Swp { byte: true },
                    0x8..=0xF if proc.arm_version().is_at_least(ArmVersion::V6) => {
                        Instr::TransferExclusive {
                            size: match instr >> 21 & 3 {
                                0 => TransferSize::Word,
                                1 if proc.has_multiprocessing() => TransferSize::Dword,
                                2 if proc.has_multiprocessing() => TransferSize::Byte,
                                3 if proc.has_multiprocessing() => TransferSize::Half,
                                _ => return Instr::Undefined { stable: false },
                            },
                            load: instr & 1 << 20 != 0,
                        }
                    }
                    _ => Instr::Undefined { stable: false },
                }
            }
        } else {
            let ty = if instr >> 5 & 3 == 1 {
                if instr & 1 << 20 == 0 {
                    MiscTransferTy::Strh
                } else {
                    MiscTransferTy::Ldrh
                }
            } else if instr & 1 << 20 == 0
                && (proc.has_enhanced_transfers() || matches!(proc, Processor::Arm7Tdmi))
            {
                // These instructions seem to exist on the ARM7TDMI, albeit by only performing
                // writeback without accessing memory
                if instr & 1 << 5 == 0 {
                    MiscTransferTy::Ldrd
                } else {
                    MiscTransferTy::Strd
                }
            } else if instr & 1 << 5 == 0 {
                MiscTransferTy::Ldrsb
            } else {
                MiscTransferTy::Ldrsh
            };
            // TODO: P == 0 && W != 0 should be unpredictable but still gets matched, check what
            // that combination is actually supposed to do
            Instr::MiscTransfer {
                ty,
                addressing: match (instr & 1 << 24 != 0, instr & 1 << 21 != 0) {
                    (false, _) => MiscAddressing::Post,
                    (true, false) => MiscAddressing::PreNoWb,
                    (true, true) => MiscAddressing::Pre,
                },
                offset_upwards: instr & 1 << 23 != 0,
                offset_imm: instr & 1 << 22 != 0,
            }
        }
    } else if instr & 0x0E00_0000 == 0x0200_0000 {
        if instr & 0x0190_0000 == 0x0100_0000 {
            // Undefined or msr imm
            if instr & 1 << 21 == 0 {
                Instr::Undefined { stable: false }
            } else {
                Instr::Msr {
                    ty: if instr & 1 << 22 == 0 && proc.has_multiprocessing() {
                        MsrTy::ImmMaybeHint
                    } else {
                        MsrTy::Imm
                    },
                    spsr: instr & 1 << 22 != 0,
                }
            }
        } else {
            // Data processing imm
            Instr::DpOp {
                ty: dp_op_ty(instr),
                set_flags: instr & 1 << 20 != 0,
                op: DpOperand::Imm,
            }
        }
    } else if instr & 0x0E00_0000 == 0x0400_0000 {
        // Load/store word/byte imm offset
        Instr::WbTransfer {
            load: instr & 1 << 20 != 0,
            byte: instr & 1 << 22 != 0,
            addressing: wb_addressing(instr),
            offset_upwards: instr & 1 << 23 != 0,
            offset: WbOff::Imm,
        }
    } else if instr & 0x0E00_0010 == 0x0600_0000 {
        // Load/store word/byte reg shift imm offset
        Instr::WbTransfer {
            load: instr & 1 << 20 != 0,
            byte: instr & 1 << 22 != 0,
            addressing: wb_addressing(instr),
            offset_upwards: instr & 1 << 23 != 0,
            offset: WbOff::Reg(shift_ty(instr)),
        }
    } else if instr & 0x0E00_0010 == 0x0600_0010 {
        // Media instructions
        if proc.arm_version().is_at_least(ArmVersion::V6) {
            match instr >> 23 & 3 {
                0 => {
                    // Parallel add/subtract
                    Instr::ParAddSub {
                        prefix: match instr >> 20 & 7 {
                            1 => ParAddSubPrefix::S,
                            2 => ParAddSubPrefix::Q,
                            3 => ParAddSubPrefix::Sh,
                            5 => ParAddSubPrefix::U,
                            6 => ParAddSubPrefix::Uq,
                            7 => ParAddSubPrefix::Uh,
                            _ => return Instr::Undefined { stable: false },
                        },
                        ty: match instr >> 5 & 7 {
                            0 => ParAddSubTy::Add16,
                            1 => ParAddSubTy::AddSubX,
                            2 => ParAddSubTy::SubAddX,
                            3 => ParAddSubTy::Sub16,
                            4 => ParAddSubTy::Add8,
                            7 => ParAddSubTy::Sub8,
                            _ => return Instr::Undefined { stable: false },
                        },
                    }
                }
                1 => {
                    if instr & 1 << 5 == 0 {
                        // Halfword pack and word saturate
                        if instr >> 20 & 7 == 0 {
                            if instr & 1 << 6 == 0 {
                                Instr::Pkhbt
                            } else {
                                Instr::Pkhtb
                            }
                        } else if instr & 1 << 21 == 0 {
                            Instr::Undefined { stable: false }
                        } else {
                            Instr::Sat {
                                par_half: false,
                                signed: instr & 1 << 22 == 0,
                            }
                        }
                    } else {
                        // Parallel half saturate, byte reverse, select bytes and sign/zero extend
                        match instr >> 6 & 3 {
                            0 => match instr >> 20 & 7 {
                                2 => Instr::Sat {
                                    par_half: true,
                                    signed: true,
                                },
                                6 => Instr::Sat {
                                    par_half: true,
                                    signed: false,
                                },
                                3 => Instr::Rev,
                                _ => Instr::Undefined { stable: false },
                            },
                            1 => Instr::Ext {
                                src: match instr >> 20 & 3 {
                                    0 => ExtSrc::ParBytes,
                                    2 => ExtSrc::Byte,
                                    3 => ExtSrc::Half,
                                    _ => return Instr::Undefined { stable: false },
                                },
                                signed: instr & 1 << 22 == 0,
                            },
                            2 => match instr >> 20 & 7 {
                                0 => Instr::Sel,
                                3 => Instr::Rev16,
                                7 => Instr::Revsh,
                                _ => Instr::Undefined { stable: false },
                            },
                            _ => Instr::Undefined { stable: false },
                        }
                    }
                }
                2 => {
                    // Extra multiplies
                    match instr >> 20 & 7 {
                        0 => match instr >> 6 & 3 {
                            0 => Instr::MediaMul(MediaMulTy::Smlad), // Also smuad
                            1 => Instr::MediaMul(MediaMulTy::Smlsd), // Also smusd
                            _ => Instr::Undefined { stable: false },
                        },
                        4 => match instr >> 6 & 3 {
                            0 => Instr::MediaMul(MediaMulTy::Smlald),
                            1 => Instr::MediaMul(MediaMulTy::Smlsld),
                            _ => Instr::Undefined { stable: false },
                        },
                        5 => match instr >> 6 & 3 {
                            0 => Instr::MediaMul(MediaMulTy::Smmla), // Also smmul
                            3 => Instr::MediaMul(MediaMulTy::Smmls),
                            _ => Instr::Undefined { stable: false },
                        },
                        _ => Instr::Undefined { stable: false },
                    }
                }
                _ => {
                    // Unsigned sum of absolute differences
                    if instr & 0x0070_00E0 == 0 {
                        Instr::Usad8 // Also usada8
                    } else {
                        Instr::Undefined { stable: false }
                    }
                }
            }
        } else {
            Instr::Undefined { stable: false }
        }
    } else if instr & 0x0E00_0000 == 0x0800_0000 {
        Instr::TransferMultiple {
            load: instr & 1 << 20 != 0,
            increment: instr & 1 << 23 != 0,
            base_excluded: instr & 1 << 24 != 0,
            writeback: instr & 1 << 21 != 0,
            s_bit: instr & 1 << 22 != 0,
        }
    } else if instr & 0x0E00_0000 == 0x0A00_0000 {
        Instr::Branch {
            link: instr & 1 << 24 != 0,
        }
    } else if instr & 0x0E00_0000 == 0x0C00_0000 {
        // TODO: What happens on architectures that don't have mcrr/mrrc with this bit combination?
        if instr & 0x01A0_0000 == 0x0000_0000 && proc.has_enhanced_transfers() {
            // TODO: Decode other addressing mode bits?
            if instr & 1 << 22 == 0 {
                Instr::Undefined { stable: false }
            } else if instr & 1 << 20 == 0 {
                Instr::Mcrr
            } else {
                Instr::Mrrc
            }
        } else if instr & 1 << 20 == 0 {
            Instr::Stc
        } else {
            Instr::Ldc
        }
    } else if instr & 0x0F00_0000 == 0x0E00_0000 {
        if instr & 1 << 4 == 0 {
            Instr::Cdp
        } else if instr & 1 << 20 == 0 {
            Instr::Mcr
        } else {
            Instr::Mrc
        }
    } else if instr & 0x0F00_0000 == 0x0F00_0000 {
        Instr::Swi
    } else {
        Instr::Undefined {
            stable: instr & 0x0FF0_00F0 == 0x0EF0_00F0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UncondInstr {
    Cps { maybe_setend: bool },
    Clrex,
    Pld,
    Srs,
    Rfe,
    BlxImm,
    Mcrr,
    Mrrc,
    Stc,
    Ldc,
    Cdp,
    Mcr,
    Mrc,
    Undefined,
}

fn decode_arm_uncond(instr: u32, proc: Processor) -> UncondInstr {
    if instr & 0x0FF0_0020 == 0x0100_0000 && proc.arm_version().is_at_least(ArmVersion::V6) {
        UncondInstr::Cps {
            maybe_setend: instr & 0x0000_00D0 == 0x0000_0000,
        }
    } else if instr & 0x0FF0_00F0 == 0x0570_0010 && proc.has_multiprocessing() {
        UncondInstr::Clrex
    } else if instr & 0x0D70_0000 == 0x0550_0000 && proc.has_enhanced_transfers() {
        UncondInstr::Pld
    } else if instr & 0x0E50_0000 == 0x0840_0000 && proc.arm_version().is_at_least(ArmVersion::V6) {
        UncondInstr::Srs
    } else if instr & 0x0E50_0000 == 0x0810_0000 && proc.arm_version().is_at_least(ArmVersion::V6) {
        UncondInstr::Rfe
    } else if instr & 0x0E00_0000 == 0x0A00_0000 {
        UncondInstr::BlxImm
    } else if instr & 0x0E00_0000 == 0x0C00_0000 {
        // TODO: What happens on ARMv5 with this bit combination? (it's unpredictable before ARMv6)
        if instr & 0x01A0_0000 == 0 && proc.arm_version().is_at_least(ArmVersion::V6) {
            if instr & 1 << 22 == 0 {
                UncondInstr::Undefined
            } else if instr & 1 << 20 == 0 {
                UncondInstr::Mcrr
            } else {
                UncondInstr::Mrrc
            }
        } else if instr & 1 << 20 == 0 {
            UncondInstr::Stc
        } else {
            UncondInstr::Ldc
        }
    } else if instr & 0x0F00_0000 == 0x0E00_0000 {
        if instr & 1 << 4 == 0 {
            UncondInstr::Cdp
        } else if instr & 1 << 20 == 0 {
            UncondInstr::Mcr
        } else {
            UncondInstr::Mrc
        }
    } else {
        UncondInstr::Undefined
    }
}

#[must_use]
pub fn cond(proc: Processor) -> [Instr; 0x1000] {
    let mut instr_table = [MaybeUninit::uninit(); 0x1000];
    for (i, instr) in instr_table.iter_mut().enumerate() {
        *instr = MaybeUninit::new(decode_arm_cond(
            ((i & 0xFF0) << 16 | (i & 0xF) << 4) as u32,
            proc,
        ));
    }
    unsafe { transmute(instr_table) }
}

#[allow(clippy::doc_markdown)]
/// # Panics
/// Panics if the given processor's architecture version is lower than ARMv5 (which introduced the
/// unconditional instruction space).
#[must_use]
pub fn uncond(proc: Processor) -> [UncondInstr; 0x1000] {
    assert!(proc.arm_version().is_at_least(ArmVersion::V5));
    let mut instr_table = [MaybeUninit::uninit(); 0x1000];
    for (i, instr) in instr_table.iter_mut().enumerate() {
        *instr = MaybeUninit::new(decode_arm_uncond(
            ((i & 0xFF0) << 16 | (i & 0xF) << 4) as u32,
            proc,
        ));
    }
    unsafe { transmute(instr_table) }
}
