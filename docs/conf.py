# Sphinx configuration for the Ruffle Together docs on Read the Docs.

project = "Ruffle Together"
author = "Multiplayer Everything"

extensions = ["myst_parser"]
source_suffix = {".md": "markdown"}
root_doc = "index"
exclude_patterns = ["_build", "fuzzing.md"]

html_theme = "sphinx_rtd_theme"
html_title = "Ruffle Together"
