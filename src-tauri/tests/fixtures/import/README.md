# Import color fixture

`adobe-rgb-test.icc` is a synthetic RGB display profile generated with Little
CMS, D65 white (`0.3127, 0.3290`), Adobe RGB primaries (`0.64, 0.33`; `0.21,
0.71`; `0.15, 0.06`) and gamma `563/256`. It contains no third-party profile
or image data. Its creation timestamp is fixed to 2000-01-01 for reproducibility.

The test compares normalized PNG and JPEG imports with independent Little CMS
sRGB references. RGB `(180, 60, 40)` in this profile converts to approximately
sRGB `(208, 57, 34)`, and `(90, 160, 120)` converts to `(0, 161, 119)`. Dropping
the profile without converting the channels would change the displayed color.
