use std::str::FromStr;

pub struct UserBool {
    pub value: bool,
}

impl From<bool> for UserBool {
    fn from(value: bool) -> Self {
        UserBool { value }
    }
}

impl From<UserBool> for bool {
    fn from(val: UserBool) -> Self {
        val.value
    }
}

impl FromStr for UserBool {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        if s == "n" || s == "no" || s == "false" {
            Ok(false.into())
        } else if s == "y" || s == "yes" || s == "true" {
            Ok(true.into())
        } else {
            Err(format!("Cannot understand {}", s))
        }
    }
}
