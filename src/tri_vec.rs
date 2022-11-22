use std::cmp::Ordering;

#[derive(Debug, Clone, Default)]
pub struct TriVec<T>
{
    pub size : usize,
    data: Vec<Vec<T>>
}

impl<T: Default + Clone> TriVec<T>
{
    fn new(&self, size: usize, default: &T) -> TriVec<T>
    {   
        let mut data: Vec<Vec<T>> = vec![Vec::<T>::new(); size];
        for (row_idx, row) in data.iter_mut().enumerate()
        {
            for _col_idx in 0..row_idx
            {
                row.push(default.clone());
            }
        }
        TriVec::<T>{size, data}
    }
    fn at(&self, x: usize, y: usize) -> &T
    {
        //The lesser index is always first
        let index = match x.cmp(&y)
        {
            Ordering::Less => (x,y),
            _ => (y,x)
        };
        assert!(index.0 < self.size);
        &self.data[index.0][index.1]
    }
    fn all_at(&self, x : usize) -> &Vec<T>
    {
        assert!(x < self.size);
        &self.data[x]
    }
}

