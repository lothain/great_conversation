# Port of the v3 page

`web/index.html` is generated: never edit it by hand. Put changes in a step file here.

    cd web && perl tools/port/port.pl tools/port/v3-source.html tools/port/globe_block.js index.html

Every `sub1` pattern must match exactly once, so a changed v3 page fails loudly.
