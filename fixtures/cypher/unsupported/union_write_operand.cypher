MATCH (n:Service) RETURN n.name AS name UNION CREATE (m:Service {name: "new"}) RETURN m.name AS name
