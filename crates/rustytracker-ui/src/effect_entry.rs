use rustytracker_core::{
    EffectCommand, INTERNAL_EFFECT_EXTENDED_BASE, INTERNAL_EFFECT_EXTENDED_MAX,
    INTERNAL_EFFECT_EXTRA_FINE_PORTA_MAX, INTERNAL_EFFECT_EXTRA_FINE_PORTA_MIN,
    INTERNAL_EFFECT_NONZERO_ARPEGGIO, XM_EFFECT_EXTENDED,
};

const EFFECT_ENTRY_MASK: u16 = 0x0fff;
const EFFECT_NIBBLE_BITS: u32 = 4;
const EFFECT_COMMAND_SHIFT: u32 = 8;
const EFFECT_OPERAND_MASK: u16 = 0x00ff;
const NIBBLE_MASK: u8 = 0x0f;

pub(crate) fn append_effect_digit(effect: EffectCommand, digit: u8) -> EffectCommand {
    let value = ((effect_to_entry_value(effect) << EFFECT_NIBBLE_BITS)
        | u16::from(digit & NIBBLE_MASK))
        & EFFECT_ENTRY_MASK;
    effect_from_entry_value(value)
}

fn effect_to_entry_value(effect: EffectCommand) -> u16 {
    let (command, operand) = if (INTERNAL_EFFECT_EXTENDED_BASE..=INTERNAL_EFFECT_EXTENDED_MAX)
        .contains(&effect.effect)
    {
        (
            XM_EFFECT_EXTENDED,
            ((effect.effect - INTERNAL_EFFECT_EXTENDED_BASE) << EFFECT_NIBBLE_BITS)
                | (effect.operand & NIBBLE_MASK),
        )
    } else if effect.effect == INTERNAL_EFFECT_NONZERO_ARPEGGIO {
        (0, effect.operand)
    } else if (INTERNAL_EFFECT_EXTRA_FINE_PORTA_MIN..=INTERNAL_EFFECT_EXTRA_FINE_PORTA_MAX)
        .contains(&effect.effect)
    {
        (effect.effect, effect.operand)
    } else {
        (effect.effect & NIBBLE_MASK, effect.operand)
    };

    (u16::from(command) << EFFECT_COMMAND_SHIFT) | u16::from(operand)
}

fn effect_from_entry_value(value: u16) -> EffectCommand {
    let raw_command = (value >> EFFECT_COMMAND_SHIFT) as u8;
    let command = raw_command & NIBBLE_MASK;
    let operand = (value & EFFECT_OPERAND_MASK) as u8;

    if command == 0 && operand == 0 {
        EffectCommand::default()
    } else if command == 0 {
        EffectCommand {
            effect: INTERNAL_EFFECT_NONZERO_ARPEGGIO,
            operand,
        }
    } else if command == XM_EFFECT_EXTENDED {
        EffectCommand {
            effect: INTERNAL_EFFECT_EXTENDED_BASE + (operand >> EFFECT_NIBBLE_BITS),
            operand: operand & NIBBLE_MASK,
        }
    } else if (INTERNAL_EFFECT_EXTRA_FINE_PORTA_MIN..=INTERNAL_EFFECT_EXTRA_FINE_PORTA_MAX)
        .contains(&raw_command)
    {
        EffectCommand {
            effect: raw_command,
            operand,
        }
    } else {
        EffectCommand {
            effect: command,
            operand,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_entry_accepts_command_and_operand_nibbles() {
        let mut effect = EffectCommand::default();
        for digit in [0x0f, 0x00, 0x06] {
            effect = append_effect_digit(effect, digit);
        }

        assert_eq!(
            effect,
            EffectCommand {
                effect: 0x0f,
                operand: 0x06,
            }
        );
    }

    #[test]
    fn effect_entry_normalizes_extended_effects() {
        let mut effect = EffectCommand::default();
        for digit in [0x0e, 0x0a, 0x01] {
            effect = append_effect_digit(effect, digit);
        }

        assert_eq!(
            effect,
            EffectCommand {
                effect: 0x3a,
                operand: 0x01,
            }
        );
        assert_eq!(effect_to_entry_value(effect), 0x0ea1);
    }

    #[test]
    fn effect_entry_normalizes_nonzero_arpeggio() {
        let mut effect = EffectCommand::default();
        for digit in [0x00, 0x03, 0x07] {
            effect = append_effect_digit(effect, digit);
        }

        assert_eq!(
            effect,
            EffectCommand {
                effect: INTERNAL_EFFECT_NONZERO_ARPEGGIO,
                operand: 0x37,
            }
        );
    }

    #[test]
    fn effect_entry_preserves_extra_fine_portamento_internal_effects() {
        let effect = EffectCommand {
            effect: INTERNAL_EFFECT_EXTRA_FINE_PORTA_MIN,
            operand: 0x05,
        };
        let value = effect_to_entry_value(effect);

        assert_eq!(value, 0x4105);
        assert_eq!(effect_from_entry_value(value), effect);
    }
}
