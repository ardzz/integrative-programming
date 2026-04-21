use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct PaginationQuery {
    #[validate(range(min = 1))]
    pub page: Option<u32>,
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<u32>,
}

impl PaginationQuery {
    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1)
    }

    pub fn per_page(&self) -> u32 {
        self.per_page.unwrap_or(10)
    }

    pub fn offset(&self) -> u32 {
        (self.page() - 1) * self.per_page()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_default_page_is_1() {
        let query = PaginationQuery {
            page: None,
            per_page: None,
        };

        assert_eq!(query.page(), 1);
        assert_eq!(query.offset(), 0);
    }

    #[test]
    fn test_default_per_page_is_10() {
        let query = PaginationQuery {
            page: None,
            per_page: None,
        };

        assert_eq!(query.per_page(), 10);
    }

    #[test]
    fn test_deserialize_from_query() {
        let query = serde_json::from_value::<PaginationQuery>(json!({
            "page": 2,
            "per_page": 25,
        }))
        .unwrap();

        assert_eq!(query.page(), 2);
        assert_eq!(query.per_page(), 25);
        assert_eq!(query.offset(), 25);
    }
}
