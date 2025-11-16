import { PlusCircleOutlined } from '@ant-design/icons';
import { Button } from 'antd';

const DIV_STYLE = {
  width: '192px',
};

export interface AddNewButtonProps {
  onClick: () => void;
}

export function AddNewButton({ onClick }: AddNewButtonProps) {
  return (
    <div style={DIV_STYLE}>
      <Button type="primary" block onClick={onClick}>
        <PlusCircleOutlined />
      </Button>
    </div>
  );
}
