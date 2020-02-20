# 0.1.8

* Fixed a panic when adding empty files.

# 0.1.7

* Fixed another bug where file paths would not be auto-converted to forward slashes...

# 0.1.6

* Fixed a bug where path hashing was incorrectly slash-insensitive, whereas it must be slash-sensitive

# 0.1.5

* Fixed a bug where hashtable collision was not properly handled when creating new archives, resulting in files missing from the archive

# 0.1.4

* `Creator.add_file()` now replaces an entry if one already exists at the given path

# 0.1.3

* `Creator.write()` now takes `self` by mutable reference instead of by value

# 0.1.2

* Fixed a bug where the sector size shift would be off-by-one, causing multi-sector files to be incorrectly read in other programs
* Fixed a bug where backwards slashes would be incorrectly converted to forward slashes in the listfile, instead of the other way around

# 0.1.1

* Removed some dead code from the library
* Added `.start()`, `.end()`, `.size()` and `.reader()` methods to `Archive` 

# 0.1.0

Initial release