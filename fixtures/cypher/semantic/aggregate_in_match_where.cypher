MATCH (n)-[:CALLS]->(m) WHERE count(m) > 3 RETURN n.name
