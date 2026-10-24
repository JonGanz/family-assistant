use crate::entities::Role;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PasswordError {
    #[error("password must be at least {min} characters")]
    TooShort { min: usize },
    #[error("password is too long (max {max} characters)")]
    TooLong { max: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PinError {
    #[error("PIN must be 4 to 6 digits")]
    Length,
    #[error("PIN must contain digits only")]
    NotDigits,
}

pub const ADULT_PASSWORD_MIN: usize = 12;
pub const CHILD_PASSWORD_MIN: usize = 4;
pub const PASSWORD_MAX: usize = 256;

/// D-031: adults 12+ characters, children 4+.
pub fn validate_password(role: Role, password: &str) -> Result<(), PasswordError> {
    let min = match role {
        Role::Adult => ADULT_PASSWORD_MIN,
        Role::Child => CHILD_PASSWORD_MIN,
    };
    let n = password.chars().count();
    if n < min {
        return Err(PasswordError::TooShort { min });
    }
    if n > PASSWORD_MAX {
        return Err(PasswordError::TooLong { max: PASSWORD_MAX });
    }
    Ok(())
}

/// 4 to 6 ASCII digits.
pub fn validate_pin(pin: &str) -> Result<(), PinError> {
    if !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(PinError::NotDigits);
    }
    if !(4..=6).contains(&pin.len()) {
        return Err(PinError::Length);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwords() {
        assert!(validate_password(Role::Child, "abcd").is_ok());
        assert_eq!(
            validate_password(Role::Child, "abc"),
            Err(PasswordError::TooShort { min: 4 })
        );
        assert!(validate_password(Role::Adult, "abcdefghijkl").is_ok());
        assert_eq!(
            validate_password(Role::Adult, "abcdefghijk"),
            Err(PasswordError::TooShort { min: 12 })
        );
        assert!(validate_password(Role::Adult, &"x".repeat(257)).is_err());
    }

    #[test]
    fn pins() {
        for ok in ["1234", "12345", "123456"] {
            assert!(validate_pin(ok).is_ok());
        }
        assert_eq!(validate_pin("123"), Err(PinError::Length));
        assert_eq!(validate_pin("1234567"), Err(PinError::Length));
        assert_eq!(validate_pin("12a4"), Err(PinError::NotDigits));
        assert_eq!(validate_pin(""), Err(PinError::Length));
    }
}
