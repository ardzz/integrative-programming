use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PageMeta {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
    pub total_pages: u32,
}

#[derive(Debug, Serialize)]
pub struct Paginated<T: Serialize> {
    pub data: Vec<T>,
    pub meta: PageMeta,
}

impl<T: Serialize> Paginated<T> {
    pub fn new(data: Vec<T>, page: u32, per_page: u32, total: u64) -> Self {
        let total_pages = if total == 0 {
            0
        } else {
            ((total as f64) / (per_page as f64)).ceil() as u32
        };

        Self {
            data,
            meta: PageMeta {
                page,
                per_page,
                total,
                total_pages,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_pages_ceil_on_partial() {
        let paginated = Paginated::new(vec![1, 2, 3, 4, 5], 1, 5, 11);

        assert_eq!(paginated.meta.total_pages, 3);
    }

    #[test]
    fn test_total_pages_zero_on_empty() {
        let paginated: Paginated<u32> = Paginated::new(vec![], 1, 10, 0);

        assert_eq!(paginated.meta.total_pages, 0);
    }
}
