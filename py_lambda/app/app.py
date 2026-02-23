import json
import boto3
import os

sqs = boto3.client('sqs')
dynamodb = boto3.resource('dynamodb')
table = dynamodb.Table('rust-cache')

def lambda_handler_poll(event, _):
    code_time = event.get("body","error receiving user code")
    print("code and time is:")
    print(code_time)


    response = table.get_item(Key={
        "job_id": code_time
        }).json()
    print(response)


    return {
        "statusCode": 200,
        "headers": {
            "Access-Control-Allow-Origin": "https://daitergg.github.io"
        },
        "body": response
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
