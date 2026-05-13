use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Page {
    pub page: u64,
    pub page_size: u64,
}

impl Page {
    pub fn new(page: u64, page_size: u64) -> Self {
        assert!(page >= 1 && page_size >= 1, "page and page_size must be >= 1");
        Self { page, page_size }
    }

    pub fn offset(&self) -> usize {
        ((self.page - 1) * self.page_size) as usize
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
    pub total_pages: u64,
    pub items: Vec<T>,
}

impl<T> Paginated<T> {
    pub fn new(page: &Page, total: u64, items: Vec<T>) -> Self {
        let total_pages = if total == 0 {
            0
        } else {
            (total + page.page_size - 1) / page.page_size
        };
        Self {
            page: page.page,
            page_size: page.page_size,
            total,
            total_pages,
            items,
        }
    }

    pub fn empty(page: &Page) -> Self {
        Self::new(page, 0, vec![])
    }
}
