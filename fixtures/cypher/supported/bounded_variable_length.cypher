MATCH p = (a:Service)-[:CALLS*1..5]->(b:Database) RETURN p
