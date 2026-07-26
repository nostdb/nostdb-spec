MATCH (n) RETURN [(n)-[:CALLS]->(m) | m.name] AS called
