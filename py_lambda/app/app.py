import json
import boto3
import os

sqs = boto3.client('sqs')
dynamodb = boto3.resource('dynamodb')
table = dynamodb.Table('rust-cache')

def lambda_handler_poll(event, _):
    code_time = json.loads(event.get("body","error receiving user code"))
    print("code and time is:")

    response = table.get_item(Key={
        "job_id": code_time['code'],
        "job_stamp": code_time['time']
        })
    try :
        item = response.get('Item')
        print(item)
    except Exception as e:
        print(f"Error getting item: {e}\n assuming entry is not created yet")
        return {
            "statusCode": 200,
            "headers": {
                "Access-Control-Allow-Origin": "https://daitergg.github.io"
            },
            "body":json.dumps({"job_status": "PROGRESS", "job_result": "0"})
        }

    return {
        "statusCode": 200,
        "headers": {
            "Access-Control-Allow-Origin": "https://daitergg.github.io"
        },
        "body":json.dumps(item)
    }
def lambda_handler_post(event, _):
    code_time = event.get("body","error receiving user code")
    print("code and time is:")
    print(code_time)
    queue_url = os.environ['QUEUE_URL']
    response = sqs.send_message(
        QueueUrl=queue_url,
        MessageBody=code_time,
        MessageGroupId='invoke',
        MessageDeduplicationId='invoke',
    )
    
    print(f"Message sent; MessageId: {response['MessageId']}")

    return {
        "statusCode": 200,
        "headers": {
            "Access-Control-Allow-Origin": "https://daitergg.github.io"
        },
        "body": "Message sent to SQS"
    }
