import json
import boto3
import os

sqs = boto3.client('sqs')
dynamodb = boto3.resource('dynamodb')
table = dynamodb.Table('rust_cache')

def lambda_handler_poll(event, _):
    try:
        code_time = json.loads(event.get("body","error receiving user code"))
        time = int(code_time['time'])
        code = code_time['code']
        print(f"code and time is: {time}, {code}")

        response = table.get_item(Key={
            "job_auth_code": code,
            "job_expire_time": time
            })
        print(f"response is {response}")
        item = response.get('Item')
        print(f"item is {item}")
        if item == None:
            return {
                "statusCode": 200,
                "headers": {
                    "Access-Control-Allow-Origin": "https://daitergg.github.io"
                },
                "body":json.dumps({"job_status": "PROGRESS", "job_result": "0"})
            }
        item['job_expire_time'] = str(item['job_expire_time'])

        return {
            "statusCode": 200,
            "headers": {
                "Access-Control-Allow-Origin": "https://daitergg.github.io"
            },
            "body":json.dumps(item)
        }
    except Exception as e:
        return {
            "statusCode": 500,
            "headers": {
                "Access-Control-Allow-Origin": "https://daitergg.github.io"
            },
            "body":str(e)
        }

def lambda_handler_post(event, _):
    post_info = event.get("body","error receiving user code")
    print("post info is:")
    print(post_info)
    queue_url = os.environ['QUEUE_URL']
    response = sqs.send_message(
        QueueUrl=queue_url,
        MessageBody=post_info,
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
