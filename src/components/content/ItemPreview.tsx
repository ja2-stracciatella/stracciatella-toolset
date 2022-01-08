import { ExclamationOutlined } from "@ant-design/icons";
import { Avatar } from "antd";
import { useMemo } from "react";
import { useImageFile } from "../../hooks/useImage";

interface ItemPreviewProps {
    graphicType: number,
    graphicIndex: number,
}

function graphicTypeToItemPrefix(graphicType: number): string {
    switch (graphicType)
    {
        case 0:
            return "GUN";
        case 1:
            return "P1ITEM";
        case 2:
            return "P2ITEM";
    }
    return "P3ITEM"
}

export function ItemPreview({ graphicType, graphicIndex }: ItemPreviewProps) {
    const graphic = useMemo(() => {
        const prefix = graphicTypeToItemPrefix(graphicType);
        return `bigitems/${prefix}${graphicIndex.toString().padStart(2, '0')}.sti`;
    }, [graphicIndex, graphicType]);
    const { data: image, error } = useImageFile(graphic);
    const additionalAvatarProps = useMemo(() => {
        if (error) {
            console.error(error);
            return { icon: <ExclamationOutlined /> }
        }
        if (!image) {
            return {};
        }
        return { src: image };
    }, [error, image]);

    if (error) {
        return <span>Error loading image: {error.toString()}</span>
    }

    return <Avatar shape="square" {...additionalAvatarProps} />;
}