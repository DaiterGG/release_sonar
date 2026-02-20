import json

def cors_handler(event, context):
    return {
        "statusCode": 200,
        "headers": {
            'Access-Control-Allow-Origin': '*',
            'Access-Control-Allow-Headers': 'Content-Type',
            'Access-Control-Allow-Methods': 'POST, GET, OPTIONS',
        },
        "body": json.dumps({
            "print": "hello",
        }),
    }
def lambda_handler(event, context):
    return {
        "statusCode": 200,
        "body": json.dumps({
            "print": "hello",
        }),
    }
