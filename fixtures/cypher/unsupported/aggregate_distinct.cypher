MATCH (n)-[:CALLS]->(m) RETURN n.name AS name, count(DISTINCT m) AS called
