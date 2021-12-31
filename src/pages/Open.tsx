import { List, Row, Col, Typography, Tooltip, Button } from "antd";
import { PlusOutlined } from "@ant-design/icons";

import { useMods } from "../state/mods";

import "./Open.css";

export function Open() {
  const { selectMod, editableMods } = useMods();
  if (!editableMods) {
    return null;
  }
  return (
    <div className="open-root">
      <Row>
        <Col span={24}>
          <h2>
            Select mod to edit
            <Tooltip title="Add new mod">
              <Button type="primary" shape="circle" icon={<PlusOutlined />} />
            </Tooltip>
          </h2>
        </Col>
      </Row>
      <Row>
        <Col span={24}>
          <List>
            {editableMods.map((mod) => (
              <List.Item key={mod.id}>
                <Typography.Link onClick={() => selectMod(mod)}>
                  {mod.name} ({mod.version})
                </Typography.Link>
              </List.Item>
            ))}
          </List>
        </Col>
      </Row>
    </div>
  );
}
