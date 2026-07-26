MATCH (n)-[:CALLS]->(m) WITH n, count(m) AS calls WHERE calls > 3 RETURN n.name, calls
