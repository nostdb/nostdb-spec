MATCH (n) RETURN [x IN n.tags WHERE x <> ""] AS kept
