import { Col, Container, ListGroup, ListGroupItem, Row } from "react-bootstrap";
import { Navigate } from "react-router";
import { useMods } from "../state/mods";

export function Open() {
  const { mods } = useMods();
  if (!mods) {
    return <Navigate replace to="/" />;
  }
  return (
    <Container>
      <Row>
        <Col>
          <h1>Open mod for editing</h1>
        </Col>
      </Row>
      <Row>
        <ListGroup>
          {mods.map((mod) => (
            <ListGroupItem key={mod.id}>{mod.name} ({mod.version})</ListGroupItem>
          ))}
        </ListGroup>
      </Row>
    </Container>
  );
}
