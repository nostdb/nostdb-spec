MATCH (n) FOREACH (x IN [1, 2] | SET n.seen = x) RETURN n
