

#[macro_export]
macro_rules! params_path {
    ( $x:expr ) => {
        
        {

use tower_lsp::jsonrpc::Result;
            let path_result = $x.text_document.uri.to_file_path();


            let Ok(path) = path_result else {

                return Result::Err(Error::new(ErrorCode::ServerError(0)))

            };

            Result::Ok(path)
        }

    };
}
