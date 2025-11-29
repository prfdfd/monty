# Cases: simple ASCII, empty, single quote, newline, tab, non-printable, backslash
(b'hello', b'', b"it's", b'l1\nl2', b'col1\tcol2', b'\x00\xff', b'back\\slash', b'\xc2\xa3100')
# Return=(b'hello', b'', b"it's", b'l1\nl2', b'col1\tcol2', b'\x00\xff', b'back\\slash', b'\xc2\xa3100')
