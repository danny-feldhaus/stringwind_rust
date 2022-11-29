use std::cmp::Ordering;

#[derive(Debug, Clone, Default)]
pub struct TriVec<T>
{
    pub size : usize,
    data: Vec<Vec<T>>,
}

impl<T: Default + Clone + 'static> TriVec<T>
{
    pub fn new(size: usize, default: &T) -> TriVec<T>
    {   
        let mut data: Vec<Vec<T>> = vec![Vec::<T>::new(); size];
        for (row_idx, row) in data.iter_mut().enumerate()
        {
            for _col_idx in 0..row_idx+1
            {
                row.push(default.clone());
            }
        }
        TriVec::<T>{size, data}
    }

    pub fn at(&self, x: usize, y: usize) -> &T
    {
        //The greater index is always first
        let index = match x.cmp(&y)
        {
            Ordering::Greater => (x,y),
            _ => (y,x)
        };
        assert!(index.0 < self.size);
        &self.data[index.0][index.1]
    }

    pub fn at_mut(&mut self, x: usize, y: usize) -> &mut T
    {
        //The greater index is always first
        let index = match x.cmp(&y)
        {
            Ordering::Greater => (x,y),
            _ => (y,x)
        };
        assert!(index.0 < self.size);
        &mut self.data[index.0][index.1]
    }
    #[allow(dead_code)]
    pub fn set(&mut self, x: usize, y:usize, value: T)
    {
        *self.at_mut(x,y) = value;
    }
    #[allow(dead_code)]
    pub fn all_at(&self, x : usize) -> &Vec<T>
    {
        assert!(x < self.size);
        &self.data[x]
    }
}