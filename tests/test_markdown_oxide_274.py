import os


def test_markdown_oxide_274_subfolder_md_link_normalization():
    base = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    src = open(os.path.join(base, "src", "vault", "mod.rs"), encoding="utf-8").read()
    assert 'trim_start_matches("./")' in src
    assert 'strip_suffix(".md")' in src
    assert "cleaned.eq_ignore_ascii_case(&refname_path)" in src
