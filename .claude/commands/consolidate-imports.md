Consolidate and organize all imports in Rust (.rs) files within the current directory and subdirectories. For
each file:

1. **Group imports** into three sections with blank lines between:
    - Standard library imports (std::*)
    - External crate imports (alphabetically sorted)
    - Internal crate imports (crate::* or super::* or self::*)

2. **Within each group**:
    - Sort imports alphabetically by the first module name
    - Combine multiple imports from the same module using nested syntax
      Example: use std::{fs, io, path::Path};
    - Remove any duplicate imports
    - Preserve important comments associated with imports

3. **Special handling**:
    - Keep #[macro_use] attributes with their imports
    - Maintain mod declarations at the top (before use statements)
    - Preserve cfg attributes on imports
    - Keep test module imports within #[cfg(test)] blocks

4. **Format consistently**:
    - One blank line between import groups
    - No blank lines within groups
    - No trailing commas in single-line nested imports
    - Use trailing commas in multi-line nested imports

5. **Order within nested imports**:
    - Alphabetize items within braces
    - Group similar items (e.g., all traits together)

Example transformation:
  ```rust
  // Before:
  use tokio::sync::mpsc::Sender;
  use std::path::Path;
  use crate::error::Result;
  use std::fs;
  use serde::{Serialize, Deserialize};
  use tokio::sync::Mutex;
  use std::io;

  // After:
  use std::{fs, io, path::Path};

  use serde::{Deserialize, Serialize};
  use tokio::sync::{mpsc::Sender, Mutex};

  use crate::error::Result;
  ```
  Process all .rs files found, excluding any in target/, .git/, or other build directories. Report the number of
  files processed and highlight any files with particularly complex import structures that may need manual review.
