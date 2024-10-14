use std::path::Path;

pub struct Vault {
    root_dir: &'static Path,
    //db: db::FileDB,
}

type Score = f64;

pub enum Embeddable {
    File,
    Heading,
}

//impl Vault {
//    pub async fn synced(self) -> anyhow::Result<Self> {
//        // root dir -> blank index
//        //
//
//        let sync: db::MSync<()> = self.db.new_msync().await;
//
//        // populate the sync with parsed files.
//        let parsed: db::MSync<md_parser::Document> = sync
//            .async_populate(|key, file_content| async { todo!() })
//            .await;
//
//        // flat map this into files and headings
//        let embeddables: db::MSync<Embeddable> = parsed.map(|document| todo!());
//
//        let embedded_syncer: db::MSync<(Embeddable, anyhow::Result<embedder::Embedding>)> =
//            embeddables.external_async_map(|embeddables| async { todo!() });
//
//        let effect: anyhow::Result<db::FileDB> = embedded_syncer.run().await;
//        let updated: db::FileDB = effect?;
//
//        Ok(Self {
//            db: updated,
//            ..self
//        })
//    }
//}

mod db;
mod embedder;
