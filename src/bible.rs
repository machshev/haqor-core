// Bible resource

#[derive(Debug)]
pub struct Bible {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nothing() {
        let _bible = Bible { name: "KJV".into() };
    }
}
