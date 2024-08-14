
mod slot;
mod blocks;
mod location;

// tests
#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, str::FromStr, sync::Arc};

    use anyhow::Context;
    use parsing::Documents;
    use rayon::prelude::*;

    use crate::blocks::{BlockCx, Blocks};

    #[test]
    fn bench() -> anyhow::Result<()> {
        let now = std::time::Instant::now();
        let path = PathBuf::from_str("/home/felix/notes")?;
        let documents = Documents::from_root_dir(&path);

        println!("Parse Documents: {:?}", now.elapsed());

        let partial_block_cx = BlockCx::new(&documents, &path);
        println!("PartialBlockCx: {:?}", now.elapsed());
        let blocks: HashMap<_, _> = documents
            .documents()
            .par_iter()
            .map(|(p, document)| {
                let block_cx = partial_block_cx(p);
                (
                    p,
                    Blocks::new(block_cx, document)
                        .context(format!("Constructing blocks for path: {p:?}"))
                        .unwrap(),
                )
            })
            .collect();

        println!("Blocks done: {:?}", now.elapsed());

        // print blocks in 2024-08-05
        // println!(
        //     "Blocks: {:#?}",
        //     documents
        //         .documents
        //         .get(&Arc::from(path.join("2024-08-05.md")))
        // );

        // println!(
        //     "Blocks: {:#?}",
        //     blocks.get(&Arc::from(path.join("2024-08-09.md")))
        // );

        assert!(blocks
            .par_iter()
            .all(|(_, blocks)| blocks.iter().all(|block| { block.is_initialized() })));

        println!("Initialized check done: {:?}", now.elapsed());

        let elapsed = now.elapsed();
        println!("Elapsed: {:?}", elapsed);
        assert!(elapsed.as_secs() < 1);

        Ok(())
    }
}
