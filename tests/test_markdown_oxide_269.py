import os


def test_markdown_oxide_269_overlaps_inline_code_spans():
    base = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    src = open(os.path.join(base, "src", "vault", "mod.rs"), encoding="utf-8").read()
    assert "fn overlaps(&self, other: &impl Rangeable) -> bool" in src
    assert "self.includes_position(other_range.start)" in src
    assert "self.includes_position(other_range.end)" in src
    assert "code_blocks.iter().any(|codeblock| codeblock.overlaps(it))" in src
