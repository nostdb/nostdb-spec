MATCH (n:Function) CALL nostdb.evidence(n) YIELD path RETURN n.name, path
