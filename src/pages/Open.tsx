import { useState } from "react";
import { Col, Container, ListGroup, ListGroupItem, Row } from "react-bootstrap";
import { Navigate } from "react-router";

import { useMods } from "../state/mods";

import './Open.css'

export function Open() {
  const [selectedMod, setSelectedMod] = useState<string | null>(null);
  const { mods } = useMods();
  if (!mods) {
    return <Navigate replace to="/" />;
  }
  return (
      <div className="open-root">
    <Container>
      <Row>
        <Col>
          <h2>Select mod to edit</h2>
        </Col>
      </Row>
      <Row>
        <Col>
          <ListGroup as="div">
            {mods.map((mod) => (
              <ListGroupItem
                as="button"
                action
                key={mod.id}
                active={selectedMod === mod.id}
                onClick={() => setSelectedMod(mod.id)}
              >
                {mod.name} ({mod.version})
              </ListGroupItem>
            ))}
          </ListGroup>
        </Col>
      </Row>
    </Container>
    </div>
  );
}
