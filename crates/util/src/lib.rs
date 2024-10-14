use tracing::error;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub trait ResultIteratorExt: Iterator {
    fn flatten_results_and_log(self) -> FlattenResultsAndLog<Self>
    where
        Self: Sized,
        Self::Item: IntoIterator;

    fn logging_flat_map<F, T, E>(self, f: F) -> LoggingFlatMap<Self, F, T, E>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Result<T, E>,
        T: std::fmt::Debug,
        E: std::fmt::Debug;
}

pub struct FlattenResultsAndLog<I: Iterator> {
    iter: I,
}

pub struct LoggingFlatMap<I, F, T, E>
where
    I: Iterator,
    F: FnMut(I::Item) -> Result<T, E>,
    T: std::fmt::Debug,
    E: std::fmt::Debug,
{
    iter: I,
    f: F,
}

impl<I, T, E> Iterator for FlattenResultsAndLog<I>
where
    I: Iterator<Item = Result<T, E>>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter.next()? {
                Ok(item) => return Some(item),
                Err(_) => {
                    error!("Error encountered");
                }
            }
        }
    }
}

impl<I, F, T, E> Iterator for LoggingFlatMap<I, F, T, E>
where
    I: Iterator,
    F: FnMut(I::Item) -> Result<T, E>,
    T: std::fmt::Debug,
    E: std::fmt::Debug,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let item = self.iter.next()?;
            match (self.f)(item) {
                Ok(mapped) => return Some(mapped),
                Err(e) => {
                    error!("Error in logging_flat_map: {:?}", e);
                }
            }
        }
    }
}

impl<I: Iterator> ResultIteratorExt for I {
    fn flatten_results_and_log(self) -> FlattenResultsAndLog<Self>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        FlattenResultsAndLog { iter: self }
    }

    fn logging_flat_map<F, T, E>(self, f: F) -> LoggingFlatMap<Self, F, T, E>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Result<T, E>,
        T: std::fmt::Debug,
        E: std::fmt::Debug,
    {
        LoggingFlatMap { iter: self, f }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn test_logging_flat_map() {
        let input = vec![1, 2, 3, 4, 5];
        let result: Vec<_> = input
            .into_iter()
            .logging_flat_map(|x| {
                if x % 2 == 0 {
                    Ok(x * 10)
                } else {
                    Err(format!("Odd number: {}", x))
                }
            })
            .collect();

        assert_eq!(result, vec![20, 40]);
        // Errors for 1, 3, and 5 will be logged
    }
}
