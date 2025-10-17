git ls-files | grep -E -v '^(target|resources|debug_out|.vscode)/|^Cargo\.lock$|^junk.rs$|^\.gitignore$' | xargs wc -l
