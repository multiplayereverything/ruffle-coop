# Sphinx configuration for the Ruffle Together docs on Read the Docs.

project = "Ruffle Together"
author = "Multiplayer Everything"
copyright = "Multiplayer Everything and the Ruffle contributors"
release = "1.0.3.9"

extensions = [
    "myst_parser",
    "sphinx_copybutton",
    "sphinx_design",
    "notfound.extension",
]
source_suffix = {".md": "markdown"}
root_doc = "index"
exclude_patterns = ["_build", "fuzzing.md"]

myst_enable_extensions = ["colon_fence", "deflist", "substitution"]
myst_substitutions = {"release": release}
myst_heading_anchors = 3

html_theme = "furo"
html_title = "Ruffle Together"
html_logo = "../assets/brand/ruffle-together-logo.png"
html_static_path = ["_static"]
html_css_files = ["custom.css"]
html_theme_options = {
    "sidebar_hide_name": True,
    "source_repository": "https://github.com/multiplayereverything/ruffle-together/",
    "source_branch": "master",
    "source_directory": "docs/",
    "light_css_variables": {
        "color-brand-primary": "#c2410c",
        "color-brand-content": "#c2410c",
    },
    "dark_css_variables": {
        "color-brand-primary": "#fb923c",
        "color-brand-content": "#fb923c",
    },
}

# Prompts and output aren't copied with the code.
copybutton_prompt_text = r"\$ |> "
copybutton_prompt_is_regexp = True
