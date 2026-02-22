import json
import boto3
import os

def lambda_handler(event, context):
    client = boto3.client('ecs')

    # Replace with your cluster name
    cluster_name = 'fargate-sonar'
    # Replace with the task definition family and revision (or just family to use the latest)
    task_definition = 'sonar-fargate:8'

    overrides = {
        'containerOverrides': [
            {
                'name': 'daiterdg/release_sonar_rust:latest',
                'environment': [
                    {
                        'name': 'LAUNCH_PARAM_USER_CODE',
                        'value': 'Hello from Lambda!'
                    }
                ]
            }
        ]
    }

    response = client.run_task(
        cluster=cluster_name,
        launchType='FARGATE',
        taskDefinition=task_definition,
        overrides=overrides,
        count=1,
        platformVersion='LATEST',
        networkConfiguration={
            'awsvpcConfiguration': {
                'subnets': [
                    'subnet-095e23bfdfda7b24f',
                ],
                'securityGroups': [
                    'sg-055134a9b7ae3ee1f'
                ],
                'assignPublicIp': 'ENABLED'
            }
        }
    )
    
    print("responce from ecs launch");
    print(response);

    return {
        "statusCode": 200,
        "headers": {
            "Access-Control-Allow-Origin": "https://daitergg.github.io"
        },
        'body': 'Task triggered: ' + response
    }
