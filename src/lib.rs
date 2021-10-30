#![warn(clippy::pedantic)]
#![allow(clippy::verbose_bit_mask, clippy::cast_possible_truncation)]

pub mod arm;
pub mod thumb;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Processor {
    Arm7Tdmi,
    Arm9Es,
    Arm11MpCore,
}

#[allow(clippy::match_same_arms)]
impl Processor {
    #[inline]
    #[must_use]
    const fn arm_version(self) -> ArmVersion {
        match self {
            Processor::Arm7Tdmi => ArmVersion::V4,
            Processor::Arm9Es => ArmVersion::V5,
            Processor::Arm11MpCore => ArmVersion::V6,
        }
    }

    #[inline]
    #[must_use]
    pub const fn has_long_multiply(self) -> bool {
        // Long multiplies, missing from ARMv*xM architectures
        match self {
            Processor::Arm7Tdmi => true,
            Processor::Arm9Es => true,
            Processor::Arm11MpCore => true,
        }
    }

    #[inline]
    #[must_use]
    pub const fn has_thumb(self) -> bool {
        // NOTE: Not necessarily Thumb support, just Thumb instructions (required since ARMv5)
        match self {
            Processor::Arm7Tdmi => true,
            Processor::Arm9Es => true,
            Processor::Arm11MpCore => true,
        }
    }

    #[inline]
    #[must_use]
    pub const fn has_thumb_blx(self) -> bool {
        // ARMv5T addition
        match self {
            Processor::Arm7Tdmi => false,
            Processor::Arm9Es => true,
            Processor::Arm11MpCore => true,
        }
    }

    #[inline]
    #[must_use]
    pub const fn has_nop_dsp_multiplies(self) -> bool {
        // On the ARM7TDMI the enhanced DSP multiplies (SMUL*/SMLA*) act as no-ops instead of
        // undefined instructions
        match self {
            Processor::Arm7Tdmi => true,
            Processor::Arm9Es => false,
            Processor::Arm11MpCore => false,
        }
    }

    #[inline]
    #[must_use]
    pub const fn has_enhanced_dsp(self) -> bool {
        // ARMv5TE/ARMv5TExP and above
        match self {
            Processor::Arm7Tdmi => false,
            Processor::Arm9Es => true,
            Processor::Arm11MpCore => true,
        }
    }

    #[inline]
    #[must_use]
    pub const fn has_enhanced_transfers(self) -> bool {
        // ARMv5TE + instrs missing from ARMv5TExP
        match self {
            Processor::Arm7Tdmi => false,
            Processor::Arm9Es => true,
            Processor::Arm11MpCore => true,
        }
    }

    #[inline]
    #[must_use]
    pub const fn has_jazelle(self) -> bool {
        // NOTE: Not necessarily Jazelle hardware acceleration, just BXJ (required since ARMv6)
        match self {
            Processor::Arm7Tdmi => false,
            Processor::Arm9Es => false,
            Processor::Arm11MpCore => true,
        }
    }

    #[inline]
    #[must_use]
    pub const fn has_multiprocessing(self) -> bool {
        // ARMv6K and above
        match self {
            Processor::Arm7Tdmi => false,
            Processor::Arm9Es => false,
            Processor::Arm11MpCore => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ArmVersion {
    V4,
    V5,
    V6,
}

impl ArmVersion {
    const fn is_at_least(self, version: ArmVersion) -> bool {
        self as usize >= version as usize
    }
}
